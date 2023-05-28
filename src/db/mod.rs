use rbatis::Rbatis;
use crate::db::tables::{Event, PoolInfo, LastSyncBlock, Token, PriceCumulativeLast, EventHash, EventStat, EventStatData, PairStatInfo};
use num::{ToPrimitive, BigUint};
use std::collections::HashMap;
use crate::watcher::event::PairEvent;
use rbatis::rbdc::decimal::Decimal;
use std::str::FromStr;
use bigdecimal::BigDecimal;
use crate::token_price::ETH_ADDRESS;
use rbatis::executor::Executor;
use rbatis::rbdc::datetime::DateTime;
use rbatis::rbdc::date::Date;
use chrono::{Utc, NaiveDate, Days};
use anyhow::format_err;

pub(crate) mod tables;
const PAGE_SIZE:i32 = 10;
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
pub(crate) async fn get_events_by_page_number(rb: &Rbatis, pg_no:i32) -> anyhow::Result<(usize,Vec<Event>)> {
    let offset = (pg_no - 1) * PAGE_SIZE;
    let events: Vec<Event> = rb
        .query_decode("select * from events where event_type != 4 order by id desc offset ? limit ? ",
                      vec![rbs::to_value!(offset),rbs::to_value!(offset)])
        .await?;
    let quo = events.len() / PAGE_SIZE as usize;
    let pg_count = if events.len() % PAGE_SIZE as usize> 0 { quo + 1 } else { quo } ;
    Ok((pg_count,events))
}
pub(crate) async fn get_events_without_time(rb: &Rbatis) -> anyhow::Result<Vec<EventHash>> {
    let events: Vec<EventHash> = rb
        .query_decode("select * from events where event_time is null order by id asc limit 10",
                      vec![])
        .await?;
    Ok(events)
}

pub(crate) async fn get_pools_day_tvl(rb: &Rbatis,day: String) -> anyhow::Result<Vec<EventStatData>> {
    let pools_tvl_day:Vec<EventStatData> = rb
        .query_decode("select pair_address,to_char(event_time,'YYYY-MM-DD') as day,amount_x,amount_y  \
            from events where id in \
            (select id from \
                (select pair_address,to_char(event_time,'YYYY-MM-DD') as d,max(event_time),\
                max(id) as id from events where event_type = 4  and \
                to_char(event_time,'YYYY-MM-DD')=? group by pair_address,d) as a\
            ) order by pair_address asc", vec![rbs::to_value!(day)])
        .await?;

    Ok(pools_tvl_day)
}

pub(crate) async fn get_pools_day_volume(rb: &Rbatis,day: String) -> anyhow::Result<Vec<EventStatData>> {
    let pools_volume_day:Vec<EventStatData> = rb
        .query_decode("select pair_address,to_char(event_time,'YYYY-MM-DD') as day,\
            sum(amount_x) as amount_x,sum(amount_y) as amount_y from events where event_type = 3 \
            and to_char(event_time,'YYYY-MM-DD') = ? group by pair_address,day order by pair_address asc",
                      vec![rbs::to_value!(day)])
        .await?;


    Ok(pools_volume_day)
}

pub(crate) async fn save_pool(rb: &mut Rbatis, pool: &PoolInfo) -> anyhow::Result<()> {
    PoolInfo::insert(rb,pool).await?;
    Ok(())
}

// pub(crate) async fn update_pool(rb: &mut Rbatis,new_pool: PoolInfo) -> anyhow::Result<()> {
//     PoolInfo::update_by_column(rb,&new_pool,"pair_id")
//         .await?;
//     Ok(())
// }

pub async fn get_all_store_pools(rb:&Rbatis ) -> anyhow::Result<Vec<PoolInfo>> {
    let pools: Vec<PoolInfo> = rb
        .query_decode("select * from pool_info",vec![])
        .await?;
    Ok(pools)
}
pub async fn get_pools_by_page_number(rb:&Rbatis,pg_no:i32 ) -> anyhow::Result<(usize,Vec<PoolInfo>)> {
    // let pools_count: i32 = rb
    //     .query_decode("select count(1) from pool_info",vec![])
    //     .await?;
    // let page_count = if pools_count % PAGE_SIZE > 0 {
    //     pools_count/PAGE_SIZE + 1
    // } else {
    //     pools_count/PAGE_SIZE
    // };
    //
    // if pg_no > page_count { return Ok(vec![]);}
    let offset = (pg_no - 1) * PAGE_SIZE;
    let pools: Vec<PoolInfo> = rb
        .query_decode("select * from pool_info order by id desc offset ? limit ? ",
                      vec![rbs::to_value!(offset),rbs::to_value!(PAGE_SIZE)])
        .await?;
    let quo = pools.len() / PAGE_SIZE as usize;
    let pg_count = if pools.len() % PAGE_SIZE as usize > 0 { quo + 1 } else { quo } ;
    Ok((pg_count,pools))
}

pub async fn get_token(rb:&Rbatis,address: String ) -> anyhow::Result<Vec<Token>> {
    let tokens: Vec<Token> = rb
        .query_decode("select * from tokens where address = ?",vec![rbs::to_value!(address)])
        .await?;
    Ok(tokens)
}
pub async fn get_token_decimals_in_pool(rb:&Rbatis,pair_address: String ) -> anyhow::Result<(i8,i8)> {
    let x_decimals: i8 = rb
        .query_decode("select t.decimals from tokens t,pools p where p.pair_address = ? \
        and p.token_x_address = t.address",vec![rbs::to_value!(pair_address.clone())])
        .await?;
    let y_decimals: i8 = rb
        .query_decode("select t.decimals from tokens t,pools p where p.pair_address = ? \
        and p.token_y_address = t.address",vec![rbs::to_value!(pair_address)])
        .await?;
    Ok((x_decimals,y_decimals))
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
pub(crate) async fn update_events_timestamp(rb: &mut Rbatis, timestamps: Vec<(i64,i32)>) -> anyhow::Result<()> {
    let mut tx = rb
        .acquire_begin()
        .await?;

    for (id,event_time) in timestamps {
        tx.exec("update events set event_time = ? where id = ?",
                vec![rbs::to_value!(DateTime::from_timestamp(event_time as i64)), rbs::to_value!(id)])
            .await?;
    }
    tx.commit().await?;
    Ok(())
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
        if let PairEvent::SyncPairEvent(sync_event) = &event {
            //Sync event
            last_synced_reserves.insert(pair_address,(sync_event.reserve0.clone(),sync_event.reserve1.clone()));
        }

        let new_count = if let Some(count) = added_events_count.get(&pair_address) {
            count + 1
        } else {
            1u32
        };
        added_events_count.insert(pair_address, new_count);
        let db_event = Event::from(event);
        db_events.push(db_event);
    }

    save_events(rb,db_events).await?;
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

pub async fn calculate_price_hour(rb: &Rbatis,pair_address: String, is_vs_usdc: bool,
                                  is_vs_token_x:bool, token_x_decimals:i32, token_y_decimals:i32
) -> anyhow::Result<Decimal> {
    let wraped_lastest_price: Option<PriceCumulativeLast> = rb
        .query_decode("select * from price_cumulative_last where pair_address =  ? \
        order by id desc limit 1",vec![rbs::to_value!(pair_address.clone())])
        .await?;
    if wraped_lastest_price.is_none() {
        return Ok(Decimal::from_str("0").unwrap());
    }
    let lastest_price = wraped_lastest_price.unwrap();
    let lastest_block_timestamp = lastest_price.block_timestamp_last;
    println!("lastest_block_timestamp is {:?}",lastest_block_timestamp);
    let wraped_base_price: Option<PriceCumulativeLast> = rb
        .query_decode("select * from price_cumulative_last where pair_address =  ?  \
            and (? - block_timestamp_last > 3600) order by id asc limit 1",
                      vec![rbs::to_value!(pair_address.clone()),rbs::to_value!(lastest_block_timestamp)])
        .await?;

    let base_price = if wraped_base_price.is_none() {
        lastest_price.clone()
    } else {
        wraped_base_price.unwrap()
    };

    println!("base price is {:?}",base_price);
    let (delta_price0,delta_timestamp,delta_price1) = (
        db_decimal_to_big!(lastest_price.price0_cumulative_last.clone().0) - db_decimal_to_big!(base_price.price0_cumulative_last.0),
        (lastest_price.block_timestamp_last - base_price.block_timestamp_last),
        db_decimal_to_big!(lastest_price.price1_cumulative_last.clone().0) - db_decimal_to_big!(base_price.price1_cumulative_last.0),
    );
    let (mut price0,mut price1) = if delta_timestamp == 0 {
        (db_decimal_to_big!(lastest_price.price0_cumulative_last.0),
         db_decimal_to_big!(lastest_price.price1_cumulative_last.0))
    } else {
        (delta_price0 / delta_timestamp,delta_price1 / delta_timestamp)
    };


    let q112 = db_decimal_to_big!(BigUint::from(2u32).pow(112));
    let x_decimals_power = BigUint::from(10u32).pow(token_x_decimals as u32);
    let y_decimals_power = BigUint::from(10u32).pow(token_y_decimals as u32);
    let decimals_quo_x2y = db_decimal_to_big!(x_decimals_power)/db_decimal_to_big!(y_decimals_power);
    let decimals_quo_y2x = db_decimal_to_big!(y_decimals_power)/db_decimal_to_big!(x_decimals_power);
    price0 = (price0/q112.clone())*decimals_quo_x2y;
    price1 = (price1/q112)*decimals_quo_y2x;

    let mut price = if is_vs_token_x { price1 } else { price0 };
    //if vs_token is ETH, it needs to be multiplied by the ratio of ETH to USDC
    if !is_vs_usdc {
        let eth_usd_price: Decimal = rb
            .query_decode("select usd_price from tokens where address =  ? ",
                          vec![rbs::to_value!(ETH_ADDRESS)])
            .await?;
        let eth_usd_price_big_decimal = BigDecimal::from_str(&eth_usd_price.0.to_string()).unwrap_or_default();
        price  *= eth_usd_price_big_decimal;
    }

    let price_round = format!("{:.18}",price);
    println!("price_round is {:?}",price_round);
    let ret_price = Decimal::from_str(&price_round).unwrap();

    Ok(ret_price)

}

pub(crate) async fn save_day_stats(rb: &mut Rbatis, stats: Vec<EventStat>) -> anyhow::Result<()> {
    let mut tx = rb
        .acquire_begin()
        .await?;
    for stat in stats {
        tx.exec("insert into event_stats (pair_address,stat_date,x_reserves,y_reserves,x_volume,\
        y_volume,usd_tvl,usd_volume) values (?,?,?,?,?,?,?,?) on conflict(pair_address,stat_date) do update set \
        x_reserves = ?,y_reserves = ?,x_volume = ?,y_volume = ?,usd_tvl = ?,usd_volume = ?",
                vec![rbs::to_value!(stat.pair_address),
                     rbs::to_value!(stat.stat_date),
                     rbs::to_value!(stat.x_reserves.clone()),
                     rbs::to_value!(stat.y_reserves.clone()),
                     rbs::to_value!(stat.x_volume.clone()),
                     rbs::to_value!(stat.y_volume.clone()),
                     rbs::to_value!(stat.usd_tvl.clone()),
                     rbs::to_value!(stat.usd_volume.clone()),
                     rbs::to_value!(stat.x_reserves.clone()),
                     rbs::to_value!(stat.y_reserves.clone()),
                     rbs::to_value!(stat.x_volume.clone()),
                     rbs::to_value!(stat.y_volume.clone()),
                     rbs::to_value!(stat.usd_tvl.clone()),
                     rbs::to_value!(stat.usd_volume.clone()),
                ]).await?;
    }
    tx.commit().await?;
    Ok(())
}
pub async fn get_unstated_days(rb:&Rbatis,confined_start_date:&String) -> anyhow::Result<Vec<String>> {
    let start_date: Option<Date> = rb
        .query_decode("select max(stat_date) from event_stats",vec![])
        .await?;
    //start from block which confined
    let date_str = if start_date.is_none() {
        confined_start_date.to_owned()
    } else {
        start_date.unwrap().0.to_string()
    };
    println!("start_date is {:?}",date_str);
    let now = Utc::now().date_naive();
    let start_date = NaiveDate::parse_from_str(&date_str,"%Y-%m-%d").unwrap();
    let mut unstated_days = Vec::new();
    unstated_days.push(now.to_string());
    if now.gt(&start_date) {
        let mut tmp_date = now;
        loop {
            let pre_day = tmp_date.checked_sub_days(Days::new(1u64)).unwrap();
            if pre_day.lt(&start_date) {
                break;
            }
            unstated_days.push(pre_day.to_string());
            tmp_date = pre_day;
        }
    }
    unstated_days.sort_unstable();
    Ok(unstated_days)
}
pub async fn get_pool_usd_price(rb:&Rbatis,pair_address: String) -> anyhow::Result<(BigDecimal,bool)> {
    //get current token_x/token_y price
    let price_x: Option<Decimal> = rb
        .query_decode("select t.usd_price from tokens t,pool_info p where \
        t.address = p.token_x_address and p.pair_address = ?",
                      vec![rbs::to_value!(pair_address.clone())])
        .await?;
    let (price,x_price) = if price_x.is_none() {

        let price_y: Option<Decimal> = rb
            .query_decode("select t.usd_price from tokens t,pool_info p where \
        t.address = p.token_y_address and p.pair_address = ?",
                          vec![rbs::to_value!(pair_address)])
            .await?;
        (price_y,false)
    } else {
        (price_x,true)
    };
    if price.is_none() {
        return Err(format_err!("can't get the price of the token x or y in the pool"));
    }
    Ok((BigDecimal::from_str(&price.unwrap().0.to_string()).unwrap(),x_price))
}
pub async fn get_pools_stat_info_by_page_number(rb:&Rbatis,pg_no:i32) -> anyhow::Result<(usize,Vec<PairStatInfo>)> {
    let offset = (pg_no - 1) * PAGE_SIZE;
    let pools_stat_info_day: Vec<PairStatInfo> = rb
        .query_decode("select p.pair_address,p.token_x_symbol,p.token_y_symbol,p.token_x_address,p.token_y_address,\
        coalesce(s.usd_tvl,0) as usd_tvl,coalesce(s.usd_volume,0) as usd_volume,\
        coalesce(s.usd_volume,0) as usd_volume_week from \
        pool_info p left join event_stats s on p.pair_address = s.pair_address \
        order by s.usd_tvl desc offset ? limit ?", vec![rbs::to_value!(offset),rbs::to_value!(PAGE_SIZE)])
        .await?;
    let mut ret = Vec::new();
    for stat_info in pools_stat_info_day {
        let pool_week_volume: (String, Decimal) = rb
            .query_decode("select pair_address,sum(usd_volume) from event_stats where \
            pair_address = ? and s.stat_date > current_date - interval '7 days'",
                          vec![rbs::to_value!(stat_info.pair_address.clone())])
            .await?;
            let pair_stat_info = PairStatInfo {
                usd_volume_week:pool_week_volume.1,
                ..stat_info
            };
        ret.push(pair_stat_info);
    }
    let quo = ret.len() / PAGE_SIZE as usize;
    let pg_count = if ret.len() % PAGE_SIZE as usize > 0 { quo + 1 } else { quo } ;
    Ok((pg_count,ret))
}

pub async fn get_all_tvl_volumes(rb:&Rbatis) -> anyhow::Result<Vec<(Date,Decimal,Decimal)>> {
    let all_tvls: Vec<(Date,Decimal,Decimal)> = rb
        .query_decode("select stat_date,sum(usd_tvl) as tvl,sum(usd_volume) as volume from event_stats \
        group by stat_date order by stat_date desc", vec![]).await?;
    Ok(all_tvls)
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
        // let count_column = "total_swap_count".to_string();
        let pair_address = "0x558038F070A802182355A0FA4807575f30076CeD".to_string();
        rb.exec("update pool_info set  total_swap_count = ? where pair_address = ?",
                vec![rbs::to_value!(count),rbs::to_value!(pair_address)])
            .await.unwrap();

    }

    #[tokio::test]
    async fn test_update_last_sync_block() {
        let rb = Rbatis::new();
        let db_url = "postgres://postgres:postgres123@localhost/backend";
        rb.init(rbdc_pg::driver::PgDriver {}, db_url).unwrap();
        let pool = rb
            .get_pool()
            .expect("get pool failed");
        pool.resize(2);

        // let event_time = 1684056594;
        // let time = DateTime::from_timestamp(event_time);
        // rb.exec("update events set event_time = ? where id = 1",
        //         vec![rbs::to_value!(time)])
        //     .await.unwrap();

        let days = match get_unstated_days(&rb,&"2023-03-28".to_string()).await {
            Err(e) => {
                println!("{:?}", e);
                Vec::new()
            },
            Ok(days) => {
                days
            }
        } ;
        println!("{:?}",days);

    }

    #[tokio::test]
    async fn test_pair_stat_info() {
        let rb = Rbatis::new();
        let db_url = "postgres://postgres:postgres123@localhost/backend";
        rb.init(rbdc_pg::driver::PgDriver {}, db_url).unwrap();
        let pool = rb
            .get_pool()
            .expect("get pool failed");
        pool.resize(2);
        let price = get_all_tvl_volumes(&rb).await.unwrap();
        println!("{:?}",price);

    }
    //
    #[tokio::test]
    async fn test_calculate_price() {
        let rb = Rbatis::new();
        let db_url = "postgres://postgres:postgres123@localhost/backend";
        rb.init(rbdc_pg::driver::PgDriver {}, db_url).unwrap();
        let pool = rb
            .get_pool()
            .expect("get pool failed");
        pool.resize(2);
        let pair_address = "fb639cd6e5b24009c3157255c315f33df0ad9302".to_string();
        let x_decimals = 18;
        let y_decimals = 18;
        let price = calculate_price_hour(&rb,pair_address,true,false,x_decimals,y_decimals).await.unwrap();
        println!("{:?}",price);

    }
    // #[tokio::test]
    // async fn test_get_stat_info() {
    //     let mut rb = Rbatis::new();
    //     let db_url = "postgres://postgres:postgres123@localhost/backend";
    //     rb.init(rbdc_pg::driver::PgDriver {}, db_url).unwrap();
    //     let pool = rb
    //         .get_pool()
    //         .expect("get pool failed");
    //     pool.resize(2);
    //     let day = Decimal::from_str("2023-05-14").unwrap();
    //     get_pools_stat_info_by_page(&rb,day,0).await.unwrap();
    //     //println!("{:?}",price);
    //
    // }
    #[test]
    fn test_price_round() {
        let price = 2776241.005739527237224783614307467819823209977418423578576313402819369935414783867599908262491226196000000000000;
        let price_round = format!("{:.2}",price);
        let ret_price = Decimal::from_str(&price_round).unwrap();
        println!("{:?}",ret_price)
    }

}