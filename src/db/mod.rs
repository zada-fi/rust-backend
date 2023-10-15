use rbatis::Rbatis;
use crate::db::tables::{Event, PoolInfo, LastSyncBlock, Token, PriceCumulativeLast, EventHash, EventStatData, PairStatInfo, EventInfo, Project, TvlStat, VolumeStat, PairTvlStatInfo, HistoryStatInfo, LaunchpadStatInfo, StoredProjectEvent, StoredLaunchpadStat};
use num::{ToPrimitive, BigUint};
use std::collections::HashMap;
use crate::watcher::event::PairEvent;
use rbatis::rbdc::decimal::Decimal;
use std::str::FromStr;
use bigdecimal::{BigDecimal, Zero};
use crate::token_price::ETH_ADDRESS;
use rbatis::executor::Executor;
use rbatis::rbdc::datetime::DateTime;
use rbatis::rbdc::date::Date;
use chrono::{Utc, NaiveDate, Days};
use anyhow::format_err;
use crate::route::launchpad::{ProjectInfo, ClaimableProject};
use serde_json::Value;
use std::ops::{Mul, Div};
use crate::watcher::watch::ChainWatcher;
use web3::Web3;
use web3::transports::Http;
use web3::types::H160;

pub(crate) mod tables;
const PAGE_SIZE:i32 = 10;
#[macro_export]
macro_rules! db_decimal_to_big {
    ($number:expr) => {
        BigDecimal::from_str(&$number.to_string()).unwrap()
    };
}

pub fn get_trim_decimals(number: BigDecimal) -> Decimal {
    let tmp = format!("{:.8}",number);
    let arry = tmp.as_bytes();
    let mut zero_len = 0;
    let str_len = arry.len();
    for i in 0..str_len {
        let r = str_len - i -1;
        if arry[r] == '.' as u8 {
            zero_len += 1;
            break;
        } else if arry[r] != '0' as u8 {
            break;
        } else {
            zero_len += 1;
        }
    }
    Decimal::from_str(&tmp[..str_len-zero_len].to_string()).unwrap()
}
pub fn get_db_trim_decimals(number: Decimal) -> Decimal {
    let num = BigDecimal::from_str(&number.0.to_string()).unwrap();
    get_trim_decimals(num)
}
pub fn get_real_amount(decimals: u8,amount: BigDecimal) ->Decimal {
        let pow_decimals = BigDecimal::from_str(&BigUint::from(10u32).pow(decimals as u32).to_string()).unwrap();
        let real_amount = amount / pow_decimals;
        get_trim_decimals(real_amount)
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

pub async fn get_last_sync_block(rb:&Rbatis,start_block: u64) -> anyhow::Result<u64> {
    let block: Vec<LastSyncBlock> = rb
        .query_decode("select block_number from last_sync_block",vec![])
        .await?;
    let number = if block.is_empty() {
        start_block
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
pub(crate) async fn get_events_by_page_number(rb: &Rbatis, pg_no:i32) -> anyhow::Result<(usize,Vec<EventInfo>)> {
    let offset = (pg_no - 1) * PAGE_SIZE;
    let events: Vec<EventInfo> = rb
        .query_decode("select e.*,t1.symbol as token_x_symbol,t2.symbol as token_y_symbol, \
        t1.decimals as token_x_decimals,t2.decimals as token_y_decimals from events e,pool_info p,\
        tokens t1,tokens t2 where \
        e.event_type != 4 and e.pair_address = p.pair_address and p.token_x_address = t1.address and \
        p.token_y_address = t2.address and e.event_time is not null \
        order by e.event_time desc offset ? limit ? ",
                      vec![rbs::to_value!(offset),rbs::to_value!(PAGE_SIZE)])
        .await?;
    let events_count: usize = rb
        .query_decode("select count(1) from events where event_type != 4 and event_time is not null",vec![]).await?;
    let quo = events_count / PAGE_SIZE as usize;
    let pg_count = if events_count % PAGE_SIZE as usize> 0 { quo + 1 } else { quo } ;
    Ok((pg_count,events))
}
pub(crate) async fn get_events_without_time(rb: &Rbatis) -> anyhow::Result<Vec<EventHash>> {
    let events: Vec<EventHash> = rb
        .query_decode("select id,tx_hash,event_type from events where event_time is null order by id asc limit 100",
                      vec![])
        .await?;
    Ok(events)
}
pub(crate) async fn get_pools_pre_day_tvl(rb: &Rbatis,day: String) -> anyhow::Result<Vec<PairTvlStatInfo>> {
    let date = Date::from_str(&day).unwrap();
    let tvl_stats: Vec<PairTvlStatInfo> = rb
        .query_decode("with tvl_ret as (
        select s.* from (select *, row_number() over (partition by tvl_stats.pair_address
        order by  tvl_stats.stat_date  desc) as group_idx
        from  tvl_stats) s where s.group_idx = 1)
        select p.pair_address,p.token_x_symbol,p.token_y_symbol,p.token_x_address,\
        p.token_y_address,coalesce(s.usd_tvl,0) as usd_tvl from \
        pool_info p left join tvl_ret s on p.pair_address = s.pair_address \
        where s.usd_tvl is not null and s.stat_date < ?", vec![rbs::to_value!(date)])
        .await?;

    Ok(tvl_stats)
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
    let offset = (pg_no - 1) * PAGE_SIZE;
    let pools: Vec<PoolInfo> = rb
        .query_decode("select * from pool_info order by id desc offset ? limit ? ",
                      vec![rbs::to_value!(offset),rbs::to_value!(PAGE_SIZE)])
        .await?;
    let pools_count: usize = rb
        .query_decode("select count(1) from pool_info",vec![]).await?;
    let quo = pools_count / PAGE_SIZE as usize;
    let pg_count = if pools_count % PAGE_SIZE as usize > 0 { quo + 1 } else { quo } ;
    let mut token_decimals = HashMap::new();
    let mut x_tokens = pools.iter().map(|p| p.token_x_address.clone()).collect::<Vec<_>>();
    let mut y_tokens = pools.iter().map(|p| p.token_y_address.clone()).collect::<Vec<_>>();
    x_tokens.append(&mut y_tokens);
    x_tokens.sort_unstable();
    x_tokens.dedup();
    for t in x_tokens {
        let decimals: i8 = rb
            .query_decode("select decimals from tokens where address = ?",vec![rbs::to_value!(t.clone())]).await?;
        token_decimals.insert(t,decimals);
    }
    let ret = pools.iter().map(|p| {
        let x_decimals = *token_decimals.get(&p.token_x_address).unwrap() as u8;
        let y_decimals = *token_decimals.get(&p.token_y_address).unwrap() as u8;
        let x_reserves = db_decimal_to_big!(p.token_x_reserves.0);
        let y_reserves = db_decimal_to_big!(p.token_y_reserves.0);
        PoolInfo {
            token_x_reserves: get_real_amount(x_decimals,x_reserves),
            token_y_reserves: get_real_amount(y_decimals,y_reserves),
            ..p.clone()
        }
    }).collect::<Vec<_>>();
    Ok((pg_count,ret))
}

pub async fn get_pools_apy(rb: &Rbatis,pools: Vec<String>) ->anyhow::Result<Vec<BigDecimal>> {
    let mut apys = Vec::new();
    for pool in pools {
        let pool_last_day_volume: HashMap<String,Decimal> = rb
            .query_decode("select coalesce(sum(usd_volume),0) as usd_volume from volume_stats where \
            pair_address = ?",
                          vec![rbs::to_value!(pool.clone())])
            .await?;
        let pool_last_day_tvl: HashMap<String,Decimal> = rb
            .query_decode("select coalesce(sum(usd_tvl),0) as usd_tvl from tvl_stats where \
            pair_address = ? order by start_date desc limit 1",
                          vec![rbs::to_value!(pool.clone())])
            .await?;
        let usd_day_volume = pool_last_day_volume.get(&"usd_volume".to_string()).unwrap().clone();
        let usd_day_volume_decimal = BigDecimal::from_str(&usd_day_volume.to_string()).unwrap_or_default();
        let usd_tvl = pool_last_day_tvl.get(&"usd_tvl".to_string()).unwrap().clone();
        let usd_tvl_decimal = BigDecimal::from_str(&usd_tvl.to_string()).unwrap_or_default();
        let apy = if !usd_tvl_decimal.is_zero() {
            usd_day_volume_decimal.div(&usd_tvl_decimal).mul(BigDecimal::from(36500))
        } else {
            BigDecimal::from(0)
        };
        apys.push(apy);
    }
    Ok(apys)
}
pub async fn get_token(rb:&Rbatis,address: String ) -> anyhow::Result<Option<Token>> {
    let token: Option<Token> = rb
        .query_decode("select * from tokens where address = ? limit 1",vec![rbs::to_value!(address)])
        .await?;
    Ok(token)
}
pub async fn get_token_decimals_in_pool(rb:&Rbatis,pair_address: String ) -> anyhow::Result<(i8,i8)> {
    let x_decimals: i8 = rb
        .query_decode("select t.decimals from tokens t,pool_info p where p.pair_address = ? \
        and p.token_x_address = t.address limit 1",vec![rbs::to_value!(pair_address.clone())])
        .await?;
    let y_decimals: i8 = rb
        .query_decode("select t.decimals from tokens t,pool_info p where p.pair_address = ? \
        and p.token_y_address = t.address limit 1",vec![rbs::to_value!(pair_address)])
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
    for (id,event_time) in timestamps {
        rb.exec("update events set event_time = ? where id = ?",
                vec![rbs::to_value!(DateTime::from_timestamp(event_time as i64)), rbs::to_value!(id)])
            .await?;
    }
    Ok(())
}
pub(crate) async fn update_from_of_add_liq_events(rb: &mut Rbatis, from_address: Vec<(i64,String)>) -> anyhow::Result<()> {
    let mut tx = rb
        .acquire_begin()
        .await?;

    for (id,from_address) in from_address {
        tx.exec("update events set from_account = ? where id = ?",
                vec![rbs::to_value!(from_address), rbs::to_value!(id)])
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
    let ret_price = Decimal::from_str(&price_round).unwrap();

    Ok(ret_price)

}

pub(crate) async fn save_day_tvl_stats(rb: &mut Rbatis, stats: Vec<TvlStat>) -> anyhow::Result<()> {
    let mut tx = rb
        .acquire_begin()
        .await?;
    for stat in stats {
        tx.exec("insert into tvl_stats (pair_address,stat_date,x_reserves,y_reserves,usd_tvl) \
        values (?,?,?,?,?) on conflict(pair_address,stat_date) do update set \
        x_reserves = ?,y_reserves = ?,usd_tvl = ?",
                vec![rbs::to_value!(stat.pair_address),
                     rbs::to_value!(stat.stat_date),
                     rbs::to_value!(stat.x_reserves.clone()),
                     rbs::to_value!(stat.y_reserves.clone()),
                     rbs::to_value!(stat.usd_tvl.clone()),
                     rbs::to_value!(stat.x_reserves.clone()),
                     rbs::to_value!(stat.y_reserves.clone()),
                     rbs::to_value!(stat.usd_tvl.clone()),
                ]).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn save_day_volume_stats(rb: &mut Rbatis, stats: Vec<VolumeStat>) -> anyhow::Result<()> {
    let mut tx = rb
        .acquire_begin()
        .await?;
    for stat in stats {
        tx.exec("insert into volume_stats (pair_address,stat_date,x_volume,y_volume,usd_volume) \
        values (?,?,?,?,?) on conflict(pair_address,stat_date) do update set \
        x_volume = ?,y_volume = ?,usd_volume = ?",
                vec![rbs::to_value!(stat.pair_address),
                     rbs::to_value!(stat.stat_date),
                     rbs::to_value!(stat.x_volume.clone()),
                     rbs::to_value!(stat.y_volume.clone()),
                     rbs::to_value!(stat.usd_volume.clone()),
                     rbs::to_value!(stat.x_volume.clone()),
                     rbs::to_value!(stat.y_volume.clone()),
                     rbs::to_value!(stat.usd_volume.clone()),
                ]).await?;
    }
    tx.commit().await?;
    Ok(())
}
pub async fn get_unstated_days(rb:&Rbatis,confined_start_date:&String) -> anyhow::Result<Vec<String>> {
    let tvl_start_date: Option<Date> = rb
        .query_decode("select max(stat_date) from tvl_stats",vec![])
        .await?;
    let volume_start_date: Option<Date> = rb
        .query_decode("select max(stat_date) from volume_stats",vec![])
        .await?;
    //start from block which confined
    let date_str = if tvl_start_date.is_none() || volume_start_date.is_none() {
        confined_start_date.to_owned()
    } else {
        let tvl_date_str = tvl_start_date.unwrap().0.to_string();
        let volume_date_str = volume_start_date.unwrap().0.to_string();
        if tvl_date_str == volume_date_str {
            tvl_date_str
        } else {
            let tvl_date = NaiveDate::parse_from_str(&tvl_date_str, "%Y-%m-%d").unwrap();
            let volume_date = NaiveDate::parse_from_str(&volume_date_str, "%Y-%m-%d").unwrap();
            if tvl_date.gt(&volume_date) {
                volume_date_str
            } else {
                tvl_date_str
            }
        }
    };
    log::info!("start_date is {:?}",date_str);
    let now = Utc::now().date_naive();
    let start_date = NaiveDate::parse_from_str(&date_str,"%Y-%m-%d").unwrap();
    let mut unstated_days = Vec::new();
    if now.ge(&start_date) {
        unstated_days.push(now.to_string());
        let mut tmp_date = now;
        loop {
            let pre_day = tmp_date.checked_sub_days(Days::new(1u64)).unwrap();
            //todo:It should be checked that the statistics have not been completed when the service is
            // abnormally terminated, and the data of start_date needs to be re-stated
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
        t.address = p.token_x_address and p.pair_address = ? limit 1",
                      vec![rbs::to_value!(pair_address.clone())])
        .await?;
    let (price,x_price) = if price_x.is_none() {

        let price_y: Option<Decimal> = rb
            .query_decode("select t.usd_price from tokens t,pool_info p where \
        t.address = p.token_y_address and p.pair_address = ? limit 1",
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
    let pools_tvl_stat: Vec<PairTvlStatInfo> = rb
        .query_decode("with tvl_ret as (
        select s.* from (select *, row_number() over (partition by tvl_stats.pair_address
        order by  tvl_stats.stat_date  desc) as group_idx
        from  tvl_stats) s where s.group_idx = 1)
        select p.pair_address,p.token_x_symbol,p.token_y_symbol,p.token_x_address,\
        p.token_y_address,coalesce(s.usd_tvl,0) as usd_tvl from \
        pool_info p left join tvl_ret s on p.pair_address = s.pair_address \
        where s.usd_tvl is not null order by s.usd_tvl desc offset ? limit ?", vec![rbs::to_value!(offset),rbs::to_value!(PAGE_SIZE)])
        .await?;
    let pools_count: usize = rb
        .query_decode("with tvl_ret as (
        select s.* from (select *, row_number() over (partition by tvl_stats.pair_address
        order by  tvl_stats.stat_date  desc) as group_idx
        from  tvl_stats) s where s.group_idx = 1)
        select count(1) from \
        pool_info p left join tvl_ret s on p.pair_address = s.pair_address \
        where s.usd_tvl is not null", vec![])
        .await?;
    let mut ret = Vec::new();
    for tvl_stat in pools_tvl_stat {
        let pool_day_volume: HashMap<String,Decimal> = rb
            .query_decode("select coalesce(sum(usd_volume),0) as total_usd_volume from volume_stats where \
            pair_address = ? and stat_date > current_date - interval '1 days' limit 1",
                          vec![rbs::to_value!(tvl_stat.pair_address.clone())])
            .await?;
        let pool_week_volume: HashMap<String,Decimal> = rb
            .query_decode("select coalesce(sum(usd_volume),0) as total_usd_volume from volume_stats where \
            pair_address = ? and stat_date > current_date - interval '7 days' limit 1",
                          vec![rbs::to_value!(tvl_stat.pair_address.clone())])
            .await?;
        let usd_day_volume = pool_day_volume.get(&"total_usd_volume".to_string()).unwrap().clone();
        let usd_day_volume_deciaml = BigDecimal::from_str(&tvl_stat.usd_tvl.to_string()).unwrap_or_default();
        let pair_stat_info = PairStatInfo {
            pair_address: tvl_stat.pair_address,
            token_x_symbol: tvl_stat.token_x_symbol,
            token_y_symbol: tvl_stat.token_y_symbol,
            token_x_address: tvl_stat.token_x_address,
            token_y_address: tvl_stat.token_y_address,
            usd_volume_week:pool_week_volume.get(&"total_usd_volume".to_string()).unwrap().clone(),
            usd_volume: usd_day_volume,
            usd_tvl: tvl_stat.usd_tvl
        };
        ret.push(pair_stat_info);
    }

    let quo = pools_count / PAGE_SIZE as usize;
    let pg_count = if pools_count % PAGE_SIZE as usize > 0 { quo + 1 } else { quo } ;
    Ok((pg_count,ret))
}

pub async fn get_all_tvls_by_day(rb:&Rbatis) -> anyhow::Result<Vec<(String,Decimal)>> {
    let all_tvls: Vec<HistoryStatInfo> = rb
        .query_decode("select * from history_stats order by stat_date desc", vec![]).await?;
    let ret = all_tvls.iter().map(|t|
        (t.stat_date.0.to_string(), t.usd_tvl.clone())).collect::<Vec<_>>();
    Ok(ret)
}
pub async fn get_all_volumes_by_day(rb:&Rbatis) -> anyhow::Result<Vec<(String,Decimal)>> {
    let all_tvls: Vec<HistoryStatInfo> = rb
        .query_decode("select * from history_stats order by stat_date desc", vec![]).await?;
    let ret = all_tvls.iter().map(|t|
        (t.stat_date.0.to_string(), t.usd_volume.clone())).collect::<Vec<_>>();
    Ok(ret)
}
pub(crate) async fn save_history_stat(rb: &mut Rbatis, stat: HistoryStatInfo) -> anyhow::Result<()> {
    rb.exec("insert into history_stats (stat_date,usd_tvl,usd_volume) \
        values (?,?,?) on conflict (stat_date) do update set \
        usd_tvl = ?,usd_volume = ?",
            vec![
                 rbs::to_value!(stat.stat_date),
                 rbs::to_value!(stat.usd_tvl.clone()),
                 rbs::to_value!(stat.usd_volume.clone()),
                 rbs::to_value!(stat.usd_tvl.clone()),
                 rbs::to_value!(stat.usd_volume.clone()),
            ]).await?;
    Ok(())
}

// pub(crate) async fn save_history_stats(rb: &Rbatis, stats: Vec<HistoryStatInfo>) -> anyhow::Result<()> {
//     let mut tx = rb
//         .acquire_begin()
//         .await?;
//
//     for stat in stats {
//         HistoryStatInfo::insert(&mut tx, &stat)
//             .await?;
//     }
//     tx.commit().await?;
//     Ok(())
// }
pub(crate) async fn save_project(rb: &mut Rbatis, project: &Project) -> anyhow::Result<()> {
    Project::insert(rb,project).await?;
    Ok(())
}
pub async fn get_project_by_name(rb:&Rbatis,project_name: String) -> anyhow::Result<Option<Project>> {
    let project: Option<Project> = rb
        .query_decode("select * from projects where project_name = ?",vec![rbs::to_value!(project_name)])
        .await?;
    Ok(project)
}
pub async fn get_project_addresses(rb:&Rbatis) -> anyhow::Result<Vec<String>> {
    let db_projects: Option<Vec<HashMap<String, String>>> = rb
        .query_decode("select distinct project_address from projects where project_address != null",vec![])
        .await?;
    if db_projects.is_none() {
        return Ok(vec![]);
    }
    let mut ret = Vec::new();
    for p in db_projects.unwrap() {
        ret.push(p.get("project_address").unwrap().to_owned());
    }
    Ok(ret)
}
pub async fn get_projects_by_page_number(rb:&Rbatis,pg_no:i32 ) -> anyhow::Result<(usize,Vec<ProjectInfo>)> {
    let offset = (pg_no - 1) * PAGE_SIZE;
    let projects: Vec<Project> = rb
        .query_decode("select * from projects where project_address is not null order by created_time desc offset ? limit ? ",
                      vec![rbs::to_value!(offset),rbs::to_value!(PAGE_SIZE)])
        .await?;
    let projects_count: usize = rb
        .query_decode("select count(1) from projects where project_address is not null",vec![]).await?;
    let quo = projects_count / PAGE_SIZE as usize;
    let pg_count = if projects_count % PAGE_SIZE as usize > 0 { quo + 1 } else { quo } ;

    let mut ret = Vec::new();
    for p in projects.iter() {
        let links_str = &p.project_links.0[..];
        let project_links: Value = serde_json::from_str(
            links_str.trim_start_matches('"')
                .trim_end_matches('"')).unwrap();
        let total_raised: Decimal = rb
            .query_decode("select sum(op_amount) as total_raised from project_events where project_address = ? and op_type = 1",
                          vec![rbs::to_value!(p.project_address.clone().unwrap())])
            .await.unwrap_or(Decimal::from_str("0").unwrap());

        let ret_decimals: Option<i16> = rb
            .query_decode("select decimals from tokens where address = ?",
                          vec![rbs::to_value!(p.project_address.clone().unwrap())])
            .await?;
        let real_raised = match ret_decimals {
            Some(decimals) => {
                let real_decimals = BigDecimal::from_str(
                    &BigUint::from(10u32).pow(decimals as u32).to_string()).unwrap();
                BigDecimal::from_str(&total_raised.0.to_string()).unwrap().div(real_decimals)
            },
            _ => BigDecimal::from(0),
        };
        let project = ProjectInfo {
            project_name: p.project_name.clone(),
            project_description: p.project_description.clone(),
            project_pic_url: p.project_pic_url.clone(),
            project_links: serde_json::from_value(project_links).unwrap(),
            project_address: p.project_address.clone().unwrap_or_default(),
            project_owner: p.project_owner.clone(),
            receive_token: p.receive_token.clone(),
            token_symbol: p.token_symbol.clone(),
            token_address: p.token_address.clone(),
            token_price_usd: p.token_price_usd.0.to_string(),
            presale_start_time: p.presale_start_time,
            presale_end_time: p.presale_end_time,
            raise_limit: p.raise_limit.clone(),
            purchased_min_limit: p.purchased_min_limit.clone(),
            purchased_max_limit: p.purchased_max_limit.clone(),
            created_time: p.created_time.to_string(),
            last_updated_time: p.last_updated_time.clone().unwrap_or(DateTime::from_timestamp(0)).unix_timestamp(),
            paused: p.paused,
            total_raised: real_raised.to_string(),
            project_title: p.project_title.clone(),
            pubsale_end_time: p.pubsale_end_time,
        };
        ret.push(project);
    };
    Ok((pg_count,ret))
}
pub(crate) async fn update_project_addresses(rb: &mut Rbatis, addresses: HashMap<String,String>)
                                             -> anyhow::Result<()> {
    // let mut tx = rb
    //     .acquire_begin()
    //     .await?;
    for (project_name,project_address) in addresses {
        let old_project = get_project_by_name(rb,project_name.clone()).await.unwrap_or_default();
        if old_project.is_none() {
            log::warn!("not found project {}",project_name);
            return Ok(());
        }
        let old_project = old_project.unwrap();
        let links_str = &old_project.project_links.0[..];
        let old_project_links: serde_json::Value = serde_json::from_str(
            links_str.trim_start_matches('"')
                .trim_end_matches('"')).unwrap();
        let new_project = Project {
            project_address: Some(project_address),
            project_links: serde_json::from_value(old_project_links).unwrap(),
            ..old_project.clone()
        };
        Project::update_by_column(rb, &new_project,"project_name").await?;
    }
    // tx.commit().await?;
    Ok(())
}
pub(crate) async fn save_project_events(rb: &mut Rbatis, events: Vec<StoredProjectEvent>) -> anyhow::Result<()> {
    let mut tx = rb
        .acquire_begin()
        .await?;
    for event in events {
        StoredProjectEvent::insert(&mut tx, &event).await?;
    }
    tx.commit().await?;
    Ok(())
}
pub async fn get_claimable_tokens_by_page_number(rb:&Rbatis,pg_no:i32,addr: String ) -> anyhow::Result<(usize,Vec<ClaimableProject>)> {
    let offset = (pg_no - 1) * PAGE_SIZE;
    let user_invest_projects:Vec<HashMap<String,String>> = rb
        .query_decode("select project_address,sum(op_amount) as invest_amount from project_events  \
         where op_user = ? and op_type = 1 group by project_address offset ? limit ? ",
                      vec![rbs::to_value!(addr.clone()),rbs::to_value!(offset),rbs::to_value!(PAGE_SIZE)])
        .await?;
    let mut ret = Vec::new();
    for user_invest_project in user_invest_projects.iter() {
        let project_address = user_invest_project.get("project_address").unwrap();
        let invest_amount = BigDecimal::from_str(
            &user_invest_project.get("invest_amount").unwrap()).unwrap();
        let project:Option<Project> = rb.query_decode("select * from projects where \
            project_address = ?",vec![rbs::to_value!(project_address)]).await?;
        //the project should exist
        if project.is_none() {
            log::error!("project {} is not exist in db",project_address);
            continue;
        }
        let project = project.unwrap();
        //todo:for simple,we only check whether the user has claimed or not
        // let claimed_amount:Decimal = rb.query_decode("select sum(op_amount) as claimed_amount from project_events  \
        //  where op_user = ? and op_type = 2 and project_address = ?",
        //                   vec![rbs::to_value!(addr.clone()),rbs::to_value!(project_address)])
        //     .await?;
        // let claimable_amount = invest_amount - claimed_amount;
        let claimed:bool = rb.query_decode("select count(1) > 1 from project_events  \
            where op_user = ? and op_type = 2 and project_address = ?",
                          vec![rbs::to_value!(addr.clone()),rbs::to_value!(project_address)])
            .await?;
        let claimable_amount = if claimed {
            BigDecimal::from(0)
        }else {
            let token_price_usd = BigDecimal::from_str(&project.token_price_usd.0.to_string()).unwrap();
            //todo:should get decimals from token contract,since use usdc as receive_token,for simple just use 18
            let pow_decimals = BigDecimal::from_str(&BigUint::from(10u32).pow(18).to_string()).unwrap();
            let real_usdc_amount = invest_amount.div(pow_decimals);
            token_price_usd.mul(real_usdc_amount)
        };
        let claimable_project = ClaimableProject {
            project_name: project.project_name,
            project_address: project_address.clone(),
            token_symbol: project.token_symbol,
            claimable_amount: claimable_amount.to_string(),
            claim_start_time: project.pubsale_end_time
        };
        ret.push(claimable_project);

    }
    let projects_count: usize = rb
        .query_decode("select count(1) from project_events where op_user = ? and op_type = 1",vec![rbs::to_value!(addr)]).await?;
    let quo = projects_count / PAGE_SIZE as usize;
    let pg_count = if projects_count % PAGE_SIZE as usize > 0 { quo + 1 } else { quo } ;

    Ok((pg_count,ret))
}
pub(crate) async fn remove_project(rb: &mut Rbatis, project_name: String)
                                             -> anyhow::Result<()> {
    rb.exec("delete from projects where project_name = ?",
            vec![rbs::to_value!(project_name)])
        .await?;
    Ok(())
}
pub(crate) async fn save_launchpad_stat_info(rb: &mut Rbatis, info: StoredLaunchpadStat) -> anyhow::Result<()> {
    StoredLaunchpadStat::insert(rb, &info).await?;
    Ok(())
}
pub async fn get_launchpad_stat_info(rb:&Rbatis) -> anyhow::Result<Option<LaunchpadStatInfo>> {
    let stat_info: Option<LaunchpadStatInfo> = rb
        .query_decode("select total_projects,total_addresses,total_raised from \
        launchpad_stat_info order by stat_time desc limit 1",
                      vec![]).await?;
    Ok(stat_info)
}
pub async fn summary_launchpad_stat_info(rb: &mut Rbatis,web3:&Web3<Http>) -> anyhow::Result<()> {
    let total_projects: usize = rb
        .query_decode("select count(1) from projects",vec![]).await?;
    let total_addresses: usize = rb
        .query_decode("select count(distinct op_user) from project_events",
                      vec![])
        .await?;
    log::info!("{:?} {:?}",total_projects,total_addresses);
    let invest_amounts: Vec<HashMap<String,String>> = rb
        .query_decode("select p.receive_token,sum(coalesce(e.op_amount,0)) as total_amount from \
        projects p left join project_events e on p.project_address = e.project_address \
        group by p.receive_token",vec![]).await?;
    log::info!("{:?}",invest_amounts);
    let mut total_invest_amount = BigDecimal::from(0);
    for invest_amount in invest_amounts.iter() {
        let receive_token = invest_amount.get("receive_token").unwrap()
            .trim_start_matches("0x").to_ascii_lowercase();
        let token_info: Option<Token> =
        rb.query_decode("select * from tokens where address = ? limit 1",
                        vec![rbs::to_value!(receive_token.clone())]).await?;
        let token_info = if token_info.is_none() {
            // if token is not exist,get token info from contract
            ChainWatcher::get_token_info(rb,web3,H160::from_str(&receive_token).unwrap()).await?
            // return Err(format_err!("Receive token not found in tokens"));
        } else {
            token_info.unwrap()
        };
        let raw_total_amount = invest_amount.get("total_amount").unwrap();
        let real_decimals = BigDecimal::from_str(
            &BigUint::from(10u32).pow(token_info.decimals as u32).to_string()).unwrap();
        let real_usd_price =
            BigDecimal::from_str(&token_info.usd_price.unwrap_or(Decimal::from_str("0").unwrap()).0.to_string()).unwrap_or_default();
        let real_total_amount = BigDecimal::from_str(&raw_total_amount).unwrap()
            .mul(real_usd_price)
            .div(real_decimals);
        total_invest_amount += real_total_amount;
    }

    let stat_info = StoredLaunchpadStat {
        stat_time: DateTime::now(),
        total_projects,
        total_addresses,
        total_raised: Decimal::from_str(&total_invest_amount.to_string()).unwrap()
    };
    save_launchpad_stat_info(rb,stat_info).await?;
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
    #[tokio::test]
    async fn test_get_pools_pre_day_stat() {
        let mut rb = Rbatis::new();
        let db_url = "postgres://postgres:postgres123@localhost/backend";
        rb.init(rbdc_pg::driver::PgDriver {}, db_url).unwrap();
        let pool = rb
            .get_pool()
            .expect("get pool failed");
        pool.resize(2);
        get_pools_pre_day_tvl(&rb,"2023-04-05".to_string()).await.unwrap();
        //println!("{:?}",price);

    }
    #[test]
    fn test_price_round() {
        let price = 2776241.005739527237224783614307467819823209977418423578576313402819369935414783867599908262491226196000000000000;
        let price_round = format!("{:.2}",price);
        let ret_price = Decimal::from_str(&price_round).unwrap();
        println!("{:?}",ret_price)
    }

    #[tokio::test]
    async fn test_update_by_column() {
        let mut rb = Rbatis::new();
        let db_url = "postgres://postgres:postgres123@localhost/backend";
        rb.init(rbdc_pg::driver::PgDriver {}, db_url).unwrap();
        let pool = rb
            .get_pool()
            .expect("get pool failed");
        pool.resize(2);
        let new_token = Token {
            address: "a1ea0b2354f5a344110af2b6ad68e75545009a03".to_string(),
            symbol: "".to_string(),
            decimals: 19,
            coingecko_id: None,
            usd_price: None
        };
        Token::update_by_column(&mut rb,&new_token,"address").await.unwrap();
        //println!("{:?}",price);

    }
}