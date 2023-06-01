// use rbdc_pg::types::decimal;
use rbatis::rbdc::decimal::Decimal;
use crate::watcher::event::PairEvent;
use std::str::FromStr;
use web3::ethabi::Uint;
use rbatis::rbdc::date::Date;
use rbatis::rbdc::datetime::DateTime;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Token {
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
    pub coingecko_id: Option<String>,
    pub usd_price: Option<Decimal>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub tx_hash: String,
    pub event_type: i8, //1:add_liq,2:swap,3:rm_liq
    pub pair_address: String,
    pub from_account: Option<String>,
    pub to_account: Option<String>,
    pub amount_x: Option<Decimal>,
    pub amount_y: Option<Decimal>,
    pub event_time: Option<DateTime>,
    pub is_swap_x2y: Option<bool>
    // pub lp_amount : Option<Decimal>
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EventInfo {
    pub tx_hash: String,
    pub event_type: i8, //1:add_liq,2:swap,3:rm_liq
    pub pair_address: String,
    pub from_account: Option<String>,
    pub to_account: Option<String>,
    pub amount_x: Option<Decimal>,
    pub amount_y: Option<Decimal>,
    pub event_time: Option<DateTime>,
    pub is_swap_x2y: Option<bool>,
    pub token_x_symbol: String,
    pub token_y_symbol: String,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EventStat {
    pub pair_address: String,
    pub stat_date: Date,
    pub x_reserves: Decimal,
    pub y_reserves: Decimal,
    pub x_volume: Decimal,
    pub y_volume: Decimal,
    pub usd_tvl: Decimal,
    pub usd_volume: Decimal,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EventStatData {
    pub pair_address: String,
    pub day: String,
    pub amount_x: Decimal,
    pub amount_y: Decimal,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EventHash {
    pub id: i64,
    pub tx_hash: String,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PoolInfo {
    // pub(crate) id: i32,
    pub(crate) pair_address: String,
    pub(crate) token_x_symbol:String,
    pub(crate) token_y_symbol: String,
    pub(crate) token_x_address: String,
    pub(crate) token_y_address: String,
    pub(crate) token_x_reserves: Decimal,
    pub(crate) token_y_reserves: Decimal,
    pub(crate) total_swap_count: i64,
    pub(crate) total_add_liq_count: i64,
    pub(crate) total_rm_liq_count: i64,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PairStatInfo {
    // pub(crate) id: i32,
    pub(crate) pair_address: String,
    pub(crate) token_x_symbol:String,
    pub(crate) token_y_symbol: String,
    pub(crate) token_x_address: String,
    pub(crate) token_y_address: String,
    pub(crate) usd_tvl: Decimal,
    pub(crate) usd_volume: Decimal,
    pub(crate) usd_volume_week: Decimal,

}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PriceCumulativeLast {
    // pub(crate) id: i32,
    pub(crate) pair_address: String,
    pub(crate) price0_cumulative_last: Decimal,
    pub(crate) price1_cumulative_last: Decimal,
    pub(crate) block_timestamp_last: i32,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LastSyncBlock {
    pub block_number: i64,
}

rbatis::crud!(Event {}, "events");
rbatis::crud!(PoolInfo {}, "pool_info");
rbatis::crud!(Token {}, "tokens");
rbatis::crud!(PriceCumulativeLast {}, "price_cumulative_last");
rbatis::crud!(LastSyncBlock {}, "last_sync_block");
rbatis::crud!(EventStat {}, "event_stats");

impl From<PairEvent> for Event {
    fn from(event: PairEvent) -> Self {
        match event {
            PairEvent::MintPairEvent(mint) => {
                Self {
                    tx_hash: hex::encode(mint.meta.tx_hash.as_bytes()),
                    event_type: 1,
                    pair_address: hex::encode(mint.meta.address.as_bytes()),
                    from_account: Some(hex::encode(mint.sender.as_bytes())),
                    to_account: None,
                    amount_x: Some(Decimal::from_str(&mint.amount0.to_string()).unwrap()),
                    amount_y: Some(Decimal::from_str(&mint.amount1.to_string()).unwrap()),
                    event_time: None,
                    is_swap_x2y: None
                }
            }
            PairEvent::BurnPairEvent(burn) => {
                Self {
                    tx_hash: hex::encode(burn.meta.tx_hash.as_bytes()),
                    event_type: 2,
                    pair_address: hex::encode(burn.meta.address.as_bytes()),
                    from_account: Some(hex::encode(burn.sender.as_bytes())),
                    to_account: Some(hex::encode(burn.to.as_bytes())),
                    amount_x: Some(Decimal::from_str(&burn.amount0.to_string()).unwrap()),
                    amount_y: Some(Decimal::from_str(&burn.amount1.to_string()).unwrap()),
                    event_time: None,
                    is_swap_x2y: None
                }
            }
            PairEvent::SwapPairEvent(swap) => {
                let (amount_x,amount_y,is_swap_x2y) = if swap.amount0in == Uint::zero() {
                    //y->x
                    (Decimal::from_str(&swap.amount0out.to_string()).unwrap(),
                    Decimal::from_str(&swap.amount1in.to_string()).unwrap(),
                    false)
                } else {
                    //x->y
                    (Decimal::from_str(&swap.amount0in.to_string()).unwrap(),
                    Decimal::from_str(&swap.amount1out.to_string()).unwrap(),
                    true)
                };
                Self {
                    tx_hash: hex::encode(swap.meta.tx_hash.as_bytes()),
                    event_type: 3,
                    pair_address: hex::encode(swap.meta.address.as_bytes()),
                    from_account: Some(hex::encode(swap.sender.as_bytes())),
                    to_account: Some(hex::encode(swap.to.as_bytes())),
                    amount_x: Some(amount_x),
                    amount_y: Some(amount_y),
                    event_time: None,
                    is_swap_x2y: Some(is_swap_x2y)
                }
            }
            PairEvent::SyncPairEvent(sync) => {
                //need get history reserves by Sync events
                Self {
                    tx_hash: hex::encode(sync.meta.tx_hash.as_bytes()),
                    event_type: 4,
                    pair_address: hex::encode(sync.meta.address.as_bytes()),
                    from_account: None,
                    to_account: None,
                    amount_x: Some(Decimal::from_str(&sync.reserve0.to_string()).unwrap()),
                    amount_y: Some(Decimal::from_str(&sync.reserve1.to_string()).unwrap()),
                    event_time: None,
                    is_swap_x2y: None
                }
            }
        }

    }
}
