use rbatis::Rbatis;
use crate::db::tables::{Event, PoolInfo, LastSyncBlock, Token};
use num::ToPrimitive;
use std::collections::HashMap;
use crate::watcher::event::PairEvent;
use rbatis::rbdc::decimal::Decimal;
use std::str::FromStr;

pub(crate) mod tables;

pub(crate) async fn upsert_last_sync_block(rb: &mut Rbatis, new_block : LastSyncBlock) -> anyhow::Result<()> {
    let block = LastSyncBlock::select_all(rb).await?;
    if block.is_empty() {
        LastSyncBlock::insert(rb,&new_block).await?;
    } else {
        LastSyncBlock::update_by_column(rb, &new_block, "block_number").await?;
    }
    Ok(())
}

pub async fn get_last_sync_block(rb:&Rbatis) -> anyhow::Result<u64> {
    let block: Vec<LastSyncBlock> = rb
        .query_decode("select block_number from last_sync_block",vec![])
        .await?;
    let number = if block.is_empty() {
        0u64
    } else {
        block[0].block_number.to_u64().unwrap()
    };
    Ok(number)
}

pub(crate) async fn save_events(rb: &Rbatis, events: Vec<Event>) -> anyhow::Result<()> {
    let mut tx = rb
        .acquire_begin()
        .await?;

    for event in events {
        Event::insert(&mut tx, &event)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn save_pool(rb: &mut Rbatis, pool: &PoolInfo) -> anyhow::Result<()> {
    PoolInfo::insert(rb,pool).await?;
    Ok(())
}

pub(crate) async fn update_pool(rb: &mut Rbatis,new_pool: PoolInfo) -> anyhow::Result<()> {
    PoolInfo::update_by_column(rb,&new_pool,"pair_id")
        .await?;
    Ok(())
}

pub async fn get_all_store_pools(rb:&Rbatis ) -> anyhow::Result<Vec<PoolInfo>> {
    let pools: Vec<PoolInfo> = rb
        .query_decode("select * from pool_info",vec![])
        .await?;
    Ok(pools)
}

pub async fn get_token(rb:&Rbatis,address: String ) -> anyhow::Result<Vec<Token>> {
    let tokens: Vec<Token> = rb
        .query_decode("select * from tokens where address = ?",vec![rbs::to_value!(address)])
        .await?;
    Ok(tokens)
}

pub(crate) async fn save_token(rb: &mut Rbatis, token: Token) -> anyhow::Result<()> {
    Token::insert(rb, &token)
        .await?;
    Ok(())
}

pub async fn get_tokens(rb:&Rbatis) -> anyhow::Result<Vec<Token>> {
    let tokens: Vec<Token> = rb
        .query_decode("select * from tokens",vec![])
        .await?;
    Ok(tokens)
}

pub async fn store_pair_events(rb: &mut Rbatis,events: Vec<PairEvent>) -> anyhow::Result<()> {
    let mut added_events_count = HashMap::new();
    let mut last_synced_reserves = HashMap::new();
    let mut db_events = Vec::new();
    if events.is_empty() {
        return Ok(());
    }
    let column_name = events[0].get_table_column_name().to_string().clone();
    for event in events {
        let pair_address = event.get_pair_address();
        match event {
            PairEvent::SyncPairEvent(sync_event) => {
                //Sync event
                last_synced_reserves.insert(pair_address,(sync_event.reserve0,sync_event.reserve1));
            }
            _ => {
                let new_count = if let Some(count) = added_events_count.get(&pair_address) {
                    count + 1
                } else {
                    1u32
                };
                added_events_count.insert(pair_address, new_count);
                let db_event = Event::from(event);
                db_events.push(db_event);
            }
        }
    }

    let mut tx = rb
        .acquire_begin()
        .await?;
    for event in db_events {
        Event::insert(&mut tx, &event)
            .await?;
    }
    //update total count by event type
    for (pair_address,count) in added_events_count {
        match &column_name[..] {
            "total_add_liq_count" => {
                rb.exec("update pool_info set total_add_liq_count = ? where pair_address = ?",
                        vec![rbs::to_value!(count), rbs::to_value!(hex::encode(pair_address.as_bytes()))])
                    .await?;
            },
            "total_rmv_liq_count" => {
                rb.exec("update pool_info set total_rmv_liq_count = ? where pair_address = ?",
                        vec![rbs::to_value!(count), rbs::to_value!(hex::encode(pair_address.as_bytes()))])
                    .await?;
            },
            "total_swap_count" => {
                rb.exec("update pool_info set total_swap_count = ? where pair_address = ?",
                        vec![rbs::to_value!(count), rbs::to_value!(hex::encode(pair_address.as_bytes()))])
                    .await?;
            },
            _ => {}

        }
    }
    //update pool reserves
    for (pair_address,(reserve_x,reserve_y)) in last_synced_reserves {
        let reserve_x_decimal = Decimal::from_str(&reserve_x.to_string()).unwrap();
        let reserve_y_decimal = Decimal::from_str(&reserve_y.to_string()).unwrap();
        rb.exec("update pool_info set token_x_reserves = ?,token_y_reserves = ? \
        where pair_address = ?", vec![rbs::to_value!(reserve_x_decimal),
                                      rbs::to_value!(reserve_y_decimal),
                                      rbs::to_value!(hex::encode(pair_address))])
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use ethabi::Uint;
    use web3::types::H160;

    #[tokio::test]
    async fn test_update_decimal() {
        let rb = Rbatis::new();
        let db_url = "postgres://postgres:postgres123@localhost/backend";
        rb.init(rbdc_pg::driver::PgDriver {}, db_url).unwrap();
        let pool = rb
            .get_pool()
            .expect("get pool failed");
        pool.resize(2);
        let reserve_x = Uint::from(12345666);
        let reserve_y = Uint::from(666666);
        let reserve_x_decimal = Decimal::from_str(&reserve_x.to_string()).unwrap();
        let reserve_y_decimal = Decimal::from_str(&reserve_y.to_string()).unwrap();
        let pair_address = H160::from_str("0x558038F070A802182355A0FA4807575f30076CeD").unwrap();
        println!("{:?}",hex::encode(pair_address));
        rb.exec("update pool_info set token_x_reserves = ?,token_y_reserves = ? \
            where pair_address = ?", vec![rbs::to_value!(reserve_x_decimal),
                                          rbs::to_value!(reserve_y_decimal),
                                          //rbs::to_value!("0x558038F070A802182355A0FA4807575f30076CeD")])
                                          rbs::Value::String(hex::encode(pair_address))])
                .await.unwrap();

    }

    #[tokio::test]
    async fn test_update_account() {
        let rb = Rbatis::new();
        let db_url = "postgres://postgres:postgres123@localhost/backend";
        rb.init(rbdc_pg::driver::PgDriver {}, db_url).unwrap();
        let pool = rb
            .get_pool()
            .expect("get pool failed");
        pool.resize(2);

        let count = 100;
        let count_column = "total_swap_count".to_string();
        let pair_address = "0x558038F070A802182355A0FA4807575f30076CeD".to_string();
        rb.exec("update pool_info set  total_swap_count = ? where pair_address = ?",
                vec![rbs::to_value!(count),rbs::to_value!(pair_address)])
            .await.unwrap();

    }

}