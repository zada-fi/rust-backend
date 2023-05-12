
use web3::types::Address;
use crate::db;
use crate::db::calculate_price_hour;
use std::time::Duration;
use std::collections::HashMap;
use bigdecimal::{BigDecimal, FromPrimitive};
use std::str::FromStr;
use rbatis::rbdc::decimal::Decimal;
use crate::config::BackendConfig;
use tokio::task::JoinHandle;

const USDC_ADDRESS: &str = "";
const ETH_ADDRESS: &str = "";
pub struct TokenPriceTask {
    db: rbatis::Rbatis,
    base_url: String,
    client: reqwest::Client
}
impl TokenPriceTask {
    // pub new()->Self {
    //
    // }
    pub async fn run_tick_price(&self) {
        println!("run_tick_price");
        let mut tx_poll = tokio::time::interval(Duration::from_secs(120));
        loop {
            tx_poll.tick().await;
            let tokens = db::get_tokens(&self.db).await?;
            for token in tokens {
                if let Err(e) = self.get_price(token).await {
                    println!("run_sync_pair_created_events error occurred {:?}", e);
                    log::error!("run_sync_pair_created_events error occurred {:?}", e);
                }
            }

        }
    }
    pub async fn get_price(&self,token_address: String) ->anyhow::Result<()>{
        let tokens = db::get_token(&db,token_address).await?;
        if tokens.is_empty() {
            return Err(format_err!("token not found"));
        }

        let token = tokens[0].clone();
        // If there is no price of this token on coingecko, it needs to be calculated
        // using the weighted average price of the pool
        let price = if let Some(coingecko_id) = token.coingecko_id {
            let simple_price_url = self
                .base_url
                .join(format!("api/v3/simple/price").as_str())
                .expect("Failed to join URL path");

            // If we use 2 day interval we will get hourly prices and not minute by minute which makes
            // response faster and smaller
            let simple_price = self
                .client
                .get(simple_price_url)
                .timeout(Duration::from_secs(120))
                .query(&[
                    ("vs_currency", "usd"),
                    ("ids", coingecko_id),
                ])
                .send()
                .await
                .map_err(|err| anyhow::format_err!("CoinGecko API request failed: {}", err))?
                .json::<HashMap<String,HashMap<String,BigDecimal>>>()
                .await
                .map_err(|err| anyhow::format_err!("Parse response data failed: {}", err))?;
            if simple_price.is_empty()  {
                BigDecimal::from(0)
            }

            let price = if let Some(p) = simple_price.get(&coingecko_id) {
                if let Some(usd) = p.get(&"usd".to_string()) {
                    usd.clone()
                } else {
                    BigDecimal::from(0)
                }
            } else {
                BigDecimal::from(0)
            };
            Decimal::from_str(&price.to_string()).unwrap()
        } else {
            //We assume that the token issued by the user will be pooled with USDC or ETH
            let all_pools = db::get_all_store_pools(&self.db).await?;
            let mut token_associated_pools = HashMap::new();
            for pool in all_pools {
                if pool.token_x_symbol != token_symbol && pool.token_y_symbol != token_symbol {
                    continue;
                }

                if pool.token_x_address == USDC_ADDRESS || pool.token_y_address == USDC_ADDRESS {
                    token_associated_pools.insert("USDC".to_string(),(pool.token_x_symbol == "USDC",pool.pair_address));
                    break;
                }
                if pool.token_x_address == ETH_ADDRESS || pool.token_y_address == ETH_ADDRESS {
                    token_associated_pools.insert("ETH".to_string(),(pool.token_x_symbol == "ETH",pool.pair_address));
                }
            }

            //ignore the pool which not include either USDC or ETH
            if token_associated_pools.is_empty() {
               return Ok(BigDecimal::from(0));
            }

            let (vs_token0,pair_address) = if token_associated_pools.contains_key(&"USDC".to_string()) {
                token_associated_pools.get(&"USDC".to_string())
            } else {
                token_associated_pools.get(&"ETH".to_string())
            };
            let (price0_hour,price1_hour) = db::calculate_price_hour(db,pair_address).await?;
            if vs_token0 {
                price0_hour
            } else {
                price1_hour
            }
        };
        db::store_price(&mut rb,token.address,price).await
    }
}

pub async fn run_tick_price(config: BackendConfig, db: rbatis::Rbatis) -> JoinHandle<()> {
    log::info!("Starting tick price!");
    let task = TokenPriceTask {
        db,
        base_url: config.coingecko_url,
        client:
    };
    tokio::spawn(task.run_tick_price())
}