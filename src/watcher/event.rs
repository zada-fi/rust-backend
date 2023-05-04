use web3::types::{H256, Log, H160};
use ethabi::{decode, ParamType, Address,Uint};
use crate::watcher::watch::ChainWatcher;

#[derive(Debug, Clone)]
pub struct EventData {
    pub address: Address,
    pub tx_hash: H256,
    // block_number: u64,
}
#[derive(Debug, Clone)]
pub struct PairCreatedEvent {
    pub token0_address: Address,
    pub token1_address: Address,
    pub pair_address: Address,
    pub all_pairs_length: Uint,
}
#[derive(Debug, Clone)]
pub struct PairMintEvent {
    pub meta: EventData,
    pub sender: Address,
    pub amount0: Uint,
    pub amount1: Uint
}
#[derive(Debug, Clone)]
pub struct PairBurnEvent {
    pub meta: EventData,
    pub sender: Address,
    pub to: Address,
    pub amount0: Uint,
    pub amount1: Uint
}
#[derive(Debug, Clone)]
pub struct PairSyncEvent {
    pub meta: EventData,
    pub reserve0: Uint,
    pub reserve1: Uint
}
#[derive(Debug, Clone)]
pub struct PairSwapEvent {
    pub meta: EventData,
    pub sender:Address,
    pub amount0In: Uint,
    pub amount1In: Uint,
    pub amount0Out: Uint,
    pub amount1Out: Uint,
    pub to: Address
}

#[derive(Debug)]
pub enum PairEvent {
    MintPairEvent(PairMintEvent),
    BurnPairEvent(PairBurnEvent),
    SwapPairEvent(PairSwapEvent),
    SyncPairEvent(PairSyncEvent),
}
pub enum EventType {
    AddLiq = 1,
    RmvLiq,
    Swap,
    Sync
}

impl PairEvent {
    pub fn get_pair_address(&self) ->Address {
        match self {
            Self::MintPairEvent(mint) => {
                mint.meta.address
            },
            Self::BurnPairEvent(burn) => {
                burn.meta.address
            },
            Self::SwapPairEvent(swap) => {
                swap.meta.address
            },
            Self::SyncPairEvent(sync) => {
                sync.meta.address
            }
        }
    }
    pub fn event_type(&self) ->EventType {
        match self {
            Self::MintPairEvent(_) => {
                EventType::AddLiq
            },
            Self::BurnPairEvent(_) => {
                EventType::RmvLiq
            },
            Self::SwapPairEvent(_) => {
                EventType::Swap
            },
            Self::SyncPairEvent(_) => {
                EventType::Sync
            }
        }
    }

    pub fn get_table_column_name(&self) -> &str {
        match self {
            Self::MintPairEvent(_) => {
                "total_add_liq_count"
            },
            Self::BurnPairEvent(_) => {
                "total_rm_liq_count"
            },
            Self::SwapPairEvent(_) => {
                "total_swap_count"
            },
            _ => ""
        }
    }
}

impl EventType {
    pub fn from_log_topic(topic: H256) -> Self {
        let topics = ChainWatcher::get_topics();
        let mint_topic = topics.get("mint").unwrap().clone();
        let burn_topic = topics.get("burn").unwrap().clone();
        let swap_topic = topics.get("swap").unwrap().clone();
        let sync_topic = topics.get("sync").unwrap().clone();
        if topic == mint_topic {
            Self::AddLiq
        } else if topic == burn_topic {
            Self::RmvLiq
        } else if topic == swap_topic {
            Self::Swap
        } else if topic == sync_topic {
            Self::Sync
        } else {
            panic!("Unreachable")
        }
    }
}
impl TryFrom<Log> for PairCreatedEvent {
    type Error = ethabi::Error;

    fn try_from(event: Log) -> Result<Self, Self::Error> {
        let mut dec_ev = decode(
            &[
                ParamType::Address,//pair_address
                ParamType::Uint(256), // all_pairs length
            ],
            &event.data.0,
        )?;
        Ok(PairCreatedEvent {
            token0_address: H160::from_slice(&event.topics[1].as_bytes()[12..]),
            token1_address: H160::from_slice(&event.topics[2].as_bytes()[12..]),
            pair_address: dec_ev[0].clone().into_address().unwrap().clone(),
            all_pairs_length: dec_ev[1].clone().into_uint().unwrap().clone(),
        })
    }
}
impl TryFrom<Log> for PairEvent {
    type Error = ethabi::Error;

    fn try_from(event: Log) -> anyhow::Result<Self, Self::Error> {
        let meta = EventData {
            address: event.address,
            tx_hash: event.transaction_hash.unwrap_or_default()
        };

        let event_type = EventType::from_log_topic(event.topics[0]);
        let pair_event = match event_type {
            EventType::AddLiq => {
                let mut dec_ev = decode(
                    &[
                        ParamType::Uint(256), // amount0
                        ParamType::Uint(256), // amount1
                    ],
                    &event.data.0,
                )?;
                PairEvent::MintPairEvent(PairMintEvent {
                    meta,
                    sender: H160::from_slice(&event.topics[1].as_bytes()[12..]),
                    amount0: dec_ev[0].clone().into_uint().unwrap(),
                    amount1: dec_ev[1].clone().into_uint().unwrap(),
                })
            },
            EventType::RmvLiq => {
                let mut dec_ev = decode(
                    &[
                        ParamType::Uint(256), // amount0
                        ParamType::Uint(256), // amount1
                    ],
                    &event.data.0,
                )?;
                PairEvent::BurnPairEvent(PairBurnEvent {
                    meta,
                    sender: H160::from_slice(&event.topics[1].as_bytes()[12..]),
                    amount0: dec_ev[0].clone().into_uint().unwrap(),
                    amount1: dec_ev[1].clone().into_uint().unwrap(),
                    to: H160::from_slice(&event.topics[2].as_bytes()[12..]),
                })
            },
            EventType::Swap => {
                let mut dec_ev = decode(
                    &[
                        ParamType::Uint(256), // amount0in
                        ParamType::Uint(256), // amount1in
                        ParamType::Uint(256), // amount0out
                        ParamType::Uint(256), // amount1out
                    ],
                    &event.data.0,
                )?;
                PairEvent::SwapPairEvent(PairSwapEvent {
                    meta,
                    sender: H160::from_slice(&event.topics[1].as_bytes()[12..]),
                    amount0In: dec_ev[0].clone().into_uint().unwrap(),
                    amount1In: dec_ev[1].clone().into_uint().unwrap(),
                    amount0Out: dec_ev[2].clone().into_uint().unwrap(),
                    amount1Out: dec_ev[3].clone().into_uint().unwrap(),
                    to: H160::from_slice(&event.topics[1].as_bytes()[12..])
                })
            },
            EventType::Sync => {
                let mut dec_ev = decode(
                    &[
                        ParamType::Uint(112), // reserve0
                        ParamType::Uint(112), // reserve1
                    ],
                    &event.data.0,
                )?;
                PairEvent::SyncPairEvent(PairSyncEvent {
                    meta,
                    reserve0: dec_ev[0].clone().into_uint().unwrap(),
                    reserve1: dec_ev[1].clone().into_uint().unwrap(),
                })
            },
            _ => {
                panic!("Not supported")
            }
        };
        Ok(pair_event)
    }
}