use rbatis::rbdc::common::datetime::DateTime;
use num::BigUint;
// use rbdc_pg::types::decimal;
use rbatis::rbdc::decimal::Decimal;
use crate::watcher::event::PairEvent;
use std::str::FromStr;
use web3::ethabi::Uint;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Token {
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub tx_hash: String,
    pub event_type: i8, //1:add_liq,2:swap,3:rm_liq
    pub pair_address: String,
    pub from_account: String,
    pub to_account: Option<String>,
    pub amount_x: Option<Decimal>,
    pub amount_y: Option<Decimal>,
    // pub lp_amount : Option<Decimal>
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
pub struct LastSyncBlock {
    pub block_number: i64,
}

rbatis::crud!(Event {}, "events");
rbatis::crud!(PoolInfo {}, "pool_info");
rbatis::crud!(Token {}, "tokens");
rbatis::crud!(LastSyncBlock {}, "last_sync_block");

impl From<PairEvent> for Event {
    fn from(event: PairEvent) -> Self {
        match event {
            PairEvent::MintPairEvent(mint) => {
                Self {
                    tx_hash: hex::encode(mint.meta.tx_hash.as_bytes()),
                    event_type: 1,
                    pair_address: hex::encode(mint.meta.address.as_bytes()),
                    from_account: hex::encode(mint.sender.as_bytes()),
                    to_account: None,
                    amount_x: Some(Decimal::from_str(&mint.amount0.to_string()).unwrap()),
                    amount_y: Some(Decimal::from_str(&mint.amount1.to_string()).unwrap()),
                }
            }
            PairEvent::BurnPairEvent(burn) => {
                Self {
                    tx_hash: hex::encode(burn.meta.tx_hash.as_bytes()),
                    event_type: 2,
                    pair_address: hex::encode(burn.meta.address.as_bytes()),
                    from_account: hex::encode(burn.sender.as_bytes()),
                    to_account: Some(hex::encode(burn.to.as_bytes())),
                    amount_x: Some(Decimal::from_str(&burn.amount0.to_string()).unwrap()),
                    amount_y: Some(Decimal::from_str(&burn.amount1.to_string()).unwrap()),
                }
            }
            PairEvent::SwapPairEvent(swap) => {
                let amount_x;
                let amount_y;

                if swap.amount0In == Uint::zero() {
                    //y->x
                    amount_x = Decimal::from_str(&swap.amount0Out.to_string()).unwrap();
                    amount_y = Decimal::from_str(&swap.amount1In.to_string()).unwrap();
                } else {
                    //x->y
                    amount_x = Decimal::from_str(&swap.amount0In.to_string()).unwrap();
                    amount_y = Decimal::from_str(&swap.amount1Out.to_string()).unwrap();
                }
                Self {
                    tx_hash: hex::encode(swap.meta.tx_hash.as_bytes()),
                    event_type: 3,
                    pair_address: hex::encode(swap.meta.address.as_bytes()),
                    from_account: hex::encode(swap.sender.as_bytes()),
                    to_account: Some(hex::encode(swap.to.as_bytes())),
                    amount_x: Some(amount_x),
                    amount_y: Some(amount_y),
                }
            }
            PairEvent::SyncPairEvent(_) => {
                //todo: sync event
                panic!("Sync event no need to store")
            }
        }

    }
}
