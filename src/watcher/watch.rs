use std::time::Duration;
use std::fmt::Debug;
use web3::{
    types::{BlockNumber, FilterBuilder, Log},
    Web3,
};
use crate::config::BackendConfig;
use crate::db::tables::{PoolInfo, Token, PriceCumulativeLast, EventHash};
use crate::db;
use web3::types::{H160, H256};
use web3::transports::Http;
use std::convert::TryFrom;
use web3::ethabi::Uint;
use std::collections::HashMap;
use std::cmp;
use tokio::task::JoinHandle;
use anyhow::format_err;
use web3::contract::{Contract, Options};
use rbatis::rbdc::decimal::Decimal;
use std::str::FromStr;
use crate::watcher::event::{ PairCreatedEvent, PairEvent};

const FACTORY_EVENTS: &str = include_str!("../abi/factory_abi.json");
const PAIR_EVENTS: &str = include_str!("../abi/pair_abi.json");

pub struct ChainWatcher {
    pub config: BackendConfig,
    pub web3: Web3<Http>,
    pub db: rbatis::Rbatis,
    pub all_pairs: Vec<H160>,
    pub pair_topics: HashMap<String,H256>,
}
impl ChainWatcher {
    // pub fn build_contract(abi_string: &str,web3_url:&str,contract_address:&str) -> Contract<Provider<Http>>{
    //     let abi:Abi = serde_json::from_str(&abi_string).unwrap();
    //     let http_client = Client::builder()
    //         .connect_timeout(std::time::Duration::from_secs(30))
    //         .timeout(std::time::Duration::from_secs(30))
    //         .build()
    //         .expect("Failed to build http client!");
    //     let url = Url::parse(web3_url).unwrap();
    //     let client = Provider::new(Http::new_with_client(url, http_client));
    //     let contract_address = Address::from_slice(&contract_address.as_bytes().to_vec()[..20]);
    //     Contract::new(contract_address, abi, client.into())
    // }
    // pub fn new_erc20_contract(config:BackendConfig,token_address:String) -> Contract<Provider<Http>> {
    //     let abi_string = r#"[ {
    //           "constant": true,
    //           "inputs": [],
    //           "name": "symbol",
    //           "outputs": [
    //             {
    //               "internalType": "string",
    //               "name": "",
    //               "type": "string"
    //             }
    //           ],
    //           "payable": false,
    //           "stateMutability": "view",
    //           "type": "function"
    //         }
    //     ]"#;
    //     Self::build_contract(abi_string,&config.remote_web3_url,&token_address)
    // }
    //
    pub async fn get_token_symbol(&mut self, address: H160) ->anyhow::Result<String> {
        //todo: use memory cache
        let abi_string = r#"[ {
              "constant": true,
              "inputs": [],
              "name": "symbol",
              "outputs": [
                {
                  "internalType": "string",
                  "name": "",
                  "type": "string"
                }
              ],
              "payable": false,
              "stateMutability": "view",
              "type": "function"
            },
            {
              "constant": true,
              "inputs": [],
              "name": "decimals",
              "outputs": [
                {
                  "internalType": "uint8",
                  "name": "",
                  "type": "uint8"
                }
              ],
              "payable": false,
              "stateMutability": "view",
              "type": "function"
            }
        ]"#;
        let token= db::get_token(&self.db,hex::encode(address.as_bytes())).await?;
        println!("get token is {:?}",token);
        let token_symbol = if token.is_empty() {
            //get from chain
            let erc20_abi = ethabi::Contract::load(abi_string.as_bytes()).unwrap();
            let erc20_contract = Contract::new(self.web3.eth(), address, erc20_abi);
            let symbol:String = erc20_contract.query("symbol",(),None, Options::default(), None)
                .await?;
            let decimals: u8 = erc20_contract.query("decimals",(),None, Options::default(), None)
                .await?;
            let new_token = Token {
                address: hex::encode(address.as_bytes()),
                symbol: symbol.clone(),
                decimals,
                coingecko_id: None,
                usd_price: None
            };
            // ignore the error
            // todo: should use another task to save in batches
            let _ret = db::save_token(&mut self.db,new_token).await;
            symbol
        } else {
            token[0].symbol.clone()
        };
        Ok(token_symbol)
    }

    pub fn get_topics() -> HashMap<String,H256> {
        let mut topics = HashMap::new();
        let factory_contract = ethabi::Contract::load(FACTORY_EVENTS.as_bytes()).unwrap();
        let create_topic = factory_contract
            .event("PairCreated")
            .expect("factory contract abi error")
            .signature();

        topics.insert(String::from("create_pair"),H256::from(create_topic.0));

        let pair_contract = ethabi::Contract::load(PAIR_EVENTS.as_bytes()).unwrap();
        let mint_topic = pair_contract
            .event("Mint")
            .expect("pair contract abi error")
            .signature();
        let burn_topic = pair_contract
            .event("Burn")
            .expect("pair contract abi error")
            .signature();
        let swap_topic = pair_contract
            .event("Swap")
            .expect("pair contract abi error")
            .signature();
        let sync_topic = pair_contract
            .event("Sync")
            .expect("pair contract abi error")
            .signature();
        topics.insert(String::from("mint"),H256::from(mint_topic.0));
        topics.insert(String::from("burn"),H256::from(burn_topic.0));
        topics.insert(String::from("swap"),H256::from(swap_topic.0));
        topics.insert(String::from("sync"),H256::from(sync_topic.0));
        topics
    }

    pub async fn get_price_cumulative_last(&mut self, pair_address: H160) ->anyhow::Result<()> {
        println!("get price of pool {:?}",pair_address);
        //get from chain
        let pair_contract_abi = ethabi::Contract::load(PAIR_EVENTS.as_bytes()).unwrap();
        let pair_contract = Contract::new(self.web3.eth(), pair_address, pair_contract_abi);
        let price0_cumulative_last:Uint = pair_contract.query("price0CumulativeLast",(),None, Options::default(), None)
            .await?;
        let price1_cumulative_last:Uint = pair_contract.query("price1CumulativeLast",(),None, Options::default(), None)
            .await?;
        let (_reserve0,_reserve1,block_timestamp_last):(Uint,Uint,Uint) = pair_contract.query("getReserves",(),None, Options::default(), None)
            .await?;
        let new_price_cumulative = PriceCumulativeLast {
            pair_address: hex::encode(pair_address),
            price0_cumulative_last: Decimal::from_str(&price0_cumulative_last.to_string()).unwrap(),
            price1_cumulative_last: Decimal::from_str(&price1_cumulative_last.to_string()).unwrap(),
            block_timestamp_last: block_timestamp_last.as_u32() as i32,
        };
        // ignore the error
        // todo: should use another task to save in batches
        db::save_price_cumulative_last(&mut self.db,new_price_cumulative).await?;
        Ok(())
    }

    pub async fn get_all_pairs_price_cumulative_last(&mut self) ->anyhow::Result<()> {
        for pair in self.all_pairs.clone() {
            self.get_price_cumulative_last(pair.clone()).await?;
        }
        Ok(())
    }

    pub async fn update_events_time(&mut self) ->anyhow::Result<()> {
        loop {
            let will_update_events:Vec<EventHash> = db::get_events_without_time(&self.db).await?;
            if will_update_events.is_empty() {
                break;
            }
            let mut update_timestamps = Vec::new();
            for event in will_update_events {
                let hash = H256::from_slice(&hex::decode(&event.tx_hash).unwrap());
                let tx = self.web3.eth().transaction(hash.into()).await?;
                let tx_time = if let Some(tx) = tx {
                    let block = self.web3.eth().block(tx.block_number.unwrap().into()).await?;
                    if let Some(block) = block {
                        block.timestamp.as_u32() as i32
                    } else { 0i32 }
                } else { 0i32 };
                println!("block time is {}",tx_time);
                update_timestamps.push((event.id,tx_time));
            }
            db::update_events_timestamp(&mut self.db,update_timestamps).await?;
        }
        Ok(())
    }
    pub async fn new(config:BackendConfig,db: rbatis::Rbatis) -> anyhow::Result<Self> {
        let transport = web3::transports::Http::new(&config.remote_web3_url).unwrap();
        let web3 = Web3::new(transport);
        let topics = Self::get_topics();
        let pools = db::get_all_store_pools(&db).await?;
        let all_pairs: Vec<H160> = pools.iter().map(|p| H160::from_str(&p.pair_address).unwrap()).collect();
        Ok(Self {
            web3,
            config,
            db,
            all_pairs,
            pair_topics:topics
        })
    }

    async fn sync_pair_created_events(
        &mut self,
        from: u64,
        to: u64,
    ) -> anyhow::Result<()> {
        let create_pair_topic = self.pair_topics.get(&String::from("create_pair")).unwrap().clone();
        println!("sync_pair_created_events {:?} {:?} {:?}",from,to,create_pair_topic);
        let logs: Vec<PairCreatedEvent> = self.sync_events(from,to,
                         vec![self.config.contract_address],
                         vec![create_pair_topic]).await?;
        // let pools = db::get_all_store_pools(&self.db).await.unwrap();
        // for pool in pools {
        //     println!("get token symbol pair address is {},token0 is {},token1 is {}",pool.pair_address,pool.token_x_address,pool.token_y_address);
        //     self.get_token_symbol(H160::from_slice(&hex::decode(pool.token_x_address).unwrap())).await.unwrap();
        //     self.get_token_symbol(H160::from_slice(&hex::decode(pool.token_y_address).unwrap())).await.unwrap();
        // }
        for event in logs {
            let token_x_symbol = self.get_token_symbol(event.token0_address).await.unwrap();
            let token_y_symbol = self.get_token_symbol(event.token1_address).await.unwrap();
            println!("Get PairCreated event : pair_address = {:?}, token0 {} address is {:?}, \
            token1 {} address is {:?}",event.pair_address.to_string(),
                     token_x_symbol,
                     hex::encode(event.token0_address),
                     token_y_symbol,
                     hex::encode(event.token1_address));
            let pool = PoolInfo {
                pair_address: hex::encode(event.pair_address),
                token_x_symbol,
                token_y_symbol,
                token_x_address: hex::encode(event.token0_address),
                token_y_address: hex::encode(event.token1_address),
                token_x_reserves: Decimal::from_str("0").unwrap(),
                token_y_reserves: Decimal::from_str("0").unwrap(),
                total_swap_count: 0,
                total_add_liq_count: 0,
                total_rm_liq_count: 0
            };

            self.all_pairs.push(event.pair_address);
            // todo: should use another task to save in batches
            db::save_pool(&mut self.db,&pool).await?;
        }

        Ok(())
    }

    async fn sync_pair_events(
        &mut self,
        from: u64,
        to: u64,
        pair_type: &str,
    ) -> anyhow::Result<()> {
        let topics = vec![self.pair_topics.get(pair_type).unwrap().clone()];
        let logs: Vec<PairEvent> = self.sync_events(from,to, self.all_pairs.clone(), topics).await?;
        if !logs.is_empty() {
            db::store_pair_events(&mut self.db, logs).await?;
        }
        Ok(())
    }
    async fn sync_events<T>(
        &mut self,
        from: u64,
        to: u64,
        address: Vec<H160>,
        topics: Vec<H256>
    ) -> anyhow::Result<Vec<T>>
    where
        T: TryFrom<Log>,
        T::Error: Debug,
    {
        let filter = FilterBuilder::default()
            .address(address)
            .from_block(BlockNumber::Number(from.into()))
            .to_block(BlockNumber::Number(to.into()))
            .topics(Some(topics), None, None, None)
            .build();
        let mut logs = self.web3.eth().logs(filter).await?;
        // println!("get logs {:?}",logs);
        let is_possible_to_sort_logs = logs.iter().all(|log| log.log_index.is_some());
        if is_possible_to_sort_logs {
            logs.sort_by_key(|log| {
                log.log_index
                    .expect("all logs log_index should have values")
            });
        } else {
            log::warn!("Some of the log entries does not have log_index, we rely on the provided logs order");
        }


        logs.into_iter()
            .map(|event| {
                T::try_from(event)
                    .map_err(|e| format_err!("Failed to parse event log from ETH: {:?}", e))
            })
            .collect()
    }

    async fn run_sync_events(&mut self) ->anyhow::Result<()> {
        let last_synced_block = db::get_last_sync_block(&self.db).await?;
        let chain_block_number = self.web3.eth().block_number().await?.as_u64();
        let sync_step = 1000u64;
        let mut start_block = last_synced_block + 1;
        let mut end_block;
        let pair_event_types = vec!["mint","burn","swap","sync"];
        loop {
            end_block = cmp::min(chain_block_number,start_block + sync_step);
            if start_block > end_block {
                break;
            }
            self.sync_pair_created_events(start_block,end_block).await?;
            for pair_event_type in &pair_event_types {
                self.sync_pair_events(start_block, end_block, pair_event_type).await?;
            }
            start_block = end_block + 1;

        }
        db::upsert_last_sync_block(
            &mut self.db,
            chain_block_number as i64,
        ).await?;
        //get price_cumulative_last info here
        self.get_all_pairs_price_cumulative_last().await?;
        //update events timestamp
        self.update_events_time().await?;
        Ok(())
    }

    pub async fn run_watcher_server(mut self) {
        println!("run_watcher_server");
        let mut tx_poll = tokio::time::interval(Duration::from_secs(1800));
        loop {
            println!("loop");
            tx_poll.tick().await;
            if let Err(e) = self.run_sync_events().await {
                println!("run_sync_pair_created_events error occurred {:?}", e);
                log::error!("run_sync_pair_created_events error occurred {:?}", e);
            }

        }
    }
}
pub async fn run_watcher(config: BackendConfig, db: rbatis::Rbatis) -> JoinHandle<()> {
    log::info!("Starting watcher!");
    let watcher = ChainWatcher::new(config, db).await.unwrap();
    tokio::spawn(watcher.run_watcher_server())
}