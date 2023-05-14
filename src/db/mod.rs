use rbatis::Rbatis;
use crate::db::tables::{Event, PoolInfo, LastSyncBlock, Token, PriceCumulativeLast};
use num::ToPrimitive;
use std::collections::HashMap;
use crate::watcher::event::PairEvent;
use rbatis::rbdc::decimal::Decimal;
use std::str::FromStr;
use bigdecimal::BigDecimal;
use crate::token_price::ETH_ADDRESS;

pub(crate) mod tables;

#[macro_export]
macro_rules! db_decimal_to_big {
    ($number:expr) => {
        BigDecimal::from_str(&$number.to_string()).unwrap()
    };
}

pub(crate) async fn upsert_last_sync_block(rb: &mut Rbatis, new_block : i64) -> anyhow::Result<()> {
    let block = LastSyncBlock::select_all(rb).await?;
    if block.is_empty() {
        rb.exec("insert into last_sync_block values (?)",
                vec![rbs::to_value!(new_block)])
            .await?;
    } else {
        rb.exec("update last_sync_block set block_number = ?",
                vec![rbs::to_value!(new_block)])
            .await?;
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
        println!("insert event tx hash {}",event.tx_hash);
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

pub async fn get_eth_price(rb:&Rbatis) -> anyhow::Result<Decimal> {
    let price: Decimal = rb
        .query_decode("select usd_price from tokens where symbol = 'ETH'",vec![])
        .await?;
    Ok(price)
}

pub async fn get_token_price(rb:&Rbatis,token_address: String) -> anyhow::Result<Decimal> {
    let price: Decimal = rb
        .query_decode("select usd_price from tokens where address = ?",vec![rbs::to_value!(token_address)])
        .await?;
    Ok(price)
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

    // let mut tx = rb
    //     .acquire_begin()
    //     .await?;
    // for event in db_events {
    //     Event::insert(&mut tx, &event)
    //         .await?;
    // }
    println!("begin save events");
    save_events(rb,db_events).await?;
    println!("end save events");
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
    println!("end update event count");
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
    println!("end update reserves");
    // tx.commit().await?;
    Ok(())
}

pub(crate) async fn save_price_cumulative_last(rb: &mut Rbatis, price: PriceCumulativeLast) -> anyhow::Result<()> {
    PriceCumulativeLast::insert(rb, &price)
        .await?;
    Ok(())
}

// pub(crate) async fn get_price_cumulative_last(rb: &mut Rbatis, price: PriceCumulativeLast) -> anyhow::Result<()> {
//     let tokens: Vec<Token> = rb
//         .query_decode("select * from tokens where address = ?",vec![rbs::to_value!(address)])
//         .await?;
//     Ok(tokens)
// }
pub(crate) async fn store_price(rb: &mut Rbatis, token_address:String,price: Decimal) -> anyhow::Result<()> {
    rb.exec("update tokens set usd_price = ? where address = ?",
            vec![rbs::to_value!(price), rbs::to_value!(token_address)])
        .await?;
    Ok(())
}

pub async fn calculate_price_hour(rb: &Rbatis,pair_address: String,is_vs_usdc: bool,is_vs_token_x:bool) -> anyhow::Result<(Decimal)> {
    let lasest_price: Vec<PriceCumulativeLast> = rb
        .query_decode("select * from price_cumulative_last where pair_address =  ? \
        order by id desc limit 1",vec![rbs::to_value!(pair_address.clone())])
        .await?;
    let mut base_price: Vec<PriceCumulativeLast> = rb
        .query_decode("select * from price_cumulative_last where pair_address =  ?  \
            and now() - block_timestamp_last > 3600 order by id asc limit 1",
                      vec![rbs::to_value!(pair_address.clone())])
        .await?;
    if lasest_price.is_empty() {
        return Ok(Decimal::from_str("0").unwrap());
    }

    if base_price.is_empty() {
        base_price = rb
            .query_decode("select * from price_cumulative_last order by id asc limit 1",
                          vec![rbs::to_value!(pair_address.clone())])
            .await?;
    }
    let (delta_price0,delta_timestamp0,delta_price1,delta_timestamp1) = (
        db_decimal_to_big!(lasest_price[0].price0_cumulative_last) - db_decimal_to_big!(base_price[0].price0_cumulative_last),
        (lasest_price[0].block_timestamp_last - base_price[0].block_timestamp_last),
        db_decimal_to_big!(lasest_price[1].price0_cumulative_last) - db_decimal_to_big!(base_price[1].price0_cumulative_last),
        (lasest_price[1].block_timestamp_last - base_price[1].block_timestamp_last)
    );
    let mut price0 = if delta_timestamp0 == 0 {
        BigDecimal::from_str(&lasest_price[0].price0_cumulative_last.clone().to_string()).unwrap()
    } else {
        delta_price0 / delta_timestamp0
    };

    let mut price1 = if delta_timestamp1 == 0 {
        BigDecimal::from_str(&lasest_price[1].price0_cumulative_last.clone().to_string()).unwrap()
    } else {
        delta_price1 / delta_timestamp1
    };

    let mut price = if is_vs_token_x { price1 } else { price0 };
    //if vs_token is ETH, it needs to be multiplied by the ratio of ETH to USDC
    if !is_vs_usdc {
        let eth_usd_price: Decimal = rb
            .query_decode("select usd_price from tokens where address =  ? ",
                          vec![rbs::to_value!(ETH_ADDRESS)])
            .await?;
        let eth_usd_price_big_decimal = BigDecimal::from_str(&eth_usd_price.to_string()).unwrap();
        price  *= eth_usd_price_big_decimal;
    }

    let ret_price = Decimal::from_str(&price.to_string()).unwrap();

    Ok(ret_price)

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

    #[tokio::test]
    async fn test_update_last_sync_block() {
        let mut rb = Rbatis::new();
        let db_url = "postgres://postgres:postgres123@localhost/backend";
        rb.init(rbdc_pg::driver::PgDriver {}, db_url).unwrap();
        let pool = rb
            .get_pool()
            .expect("get pool failed");
        pool.resize(2);

        let block_number = 1234;
        rb.exec("insert into last_sync_block values (?)",
                vec![rbs::to_value!(block_number)])
            .await.unwrap();
    }

}