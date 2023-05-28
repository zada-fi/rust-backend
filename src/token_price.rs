use crate::db;
use std::time::Duration;
use std::collections::HashMap;
use std::str::FromStr;
use rbatis::rbdc::decimal::Decimal;
use crate::config::BackendConfig;
use tokio::task::JoinHandle;
use anyhow::format_err;
use reqwest::Url;

pub const USDC_ADDRESS: &str = "a0d71b9877f44c744546d649147e3f1e70a93760";
pub const ETH_ADDRESS: &str = "a1ea0b2354f5a344110af2b6ad68e75545009a03";
pub struct TokenPriceTask {
    pub db: rbatis::Rbatis,
    pub base_url: Url,
    pub client: reqwest::Client
}
impl TokenPriceTask {
    pub fn new(db:rbatis::Rbatis,base_url:Url,client:reqwest::Client)->Self {
        Self {
            db,
            base_url,
            client
        }
    }
    pub async fn run_tick_price(mut self) {
        println!("run_tick_price");
        let mut tx_poll = tokio::time::interval(Duration::from_secs(120));
        loop {
            tx_poll.tick().await;
            let tokens = db::get_tokens(&self.db).await.unwrap_or_default();
            for token in tokens {
                if let Err(e) = self.get_price(token.address).await {
                    println!("run tick price error occurred {:?}", e);
                    log::error!("run_sync_pair_created_events error occurred {:?}", e);
                }
            }

        }
    }
    pub async fn get_price(&mut self, token_address: String) ->anyhow::Result<()>{
        let tokens = db::get_token(&self.db,token_address.clone()).await?;
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
                    ("vs_currencies", "usd"),
                    ("ids", &coingecko_id),
                ])
                .send()
                .await
                .map_err(|err| anyhow::format_err!("CoinGecko API request failed: {}", err))?
                .json::<HashMap<String,HashMap<String,f64>>>()
                .await
                .map_err(|err| anyhow::format_err!("Parse response data failed: {}", err))?;
            if simple_price.is_empty()  {
                log::info!("Response is empty, {}",token_address.clone());
                return Ok(());
            }

            let price = if let Some(p) = simple_price.get(&coingecko_id) {
                if let Some(usd) = p.get(&"usd".to_string()) {
                    usd.clone()
                } else {
                    0.0
                }
            } else {
                0.0
            };
            Decimal::from_str(&price.to_string()).unwrap()
        } else {
            //We assume that the token issued by the user will be pooled with USDC or ETH
            let all_pools = db::get_all_store_pools(&self.db).await?;
            let mut token_associated_pools: HashMap<String,(bool,String,String,String)> = HashMap::new();
            for pool in all_pools {
                if pool.token_x_address != token_address.clone() && pool.token_y_address != token_address.clone() {
                    continue;
                }

                if pool.token_x_address == USDC_ADDRESS || pool.token_y_address == USDC_ADDRESS {
                    token_associated_pools.insert("USDC".to_string(),
                                                  (pool.token_x_symbol == "USDC",
                                                      pool.pair_address,
                                                      pool.token_x_address,
                                                      pool.token_y_address,
                                                  ));
                    break;
                }
                if pool.token_x_address == ETH_ADDRESS || pool.token_y_address == ETH_ADDRESS {
                    token_associated_pools.insert("ETH".to_string(),
                                                  (pool.token_x_symbol == "ETH",
                                                      pool.pair_address,
                                                      pool.token_x_address,
                                                      pool.token_y_address,
                                                  )
                    );
                }
            }

            //ignore the pool which not include either USDC or ETH
            if token_associated_pools.is_empty() {
                log::info!("Maybe unimportant tokens, {}",token_address.clone());
                return Ok(());
            }

            let (is_usdc,vs_token0,pair_address,token_x,token_y) = if token_associated_pools.contains_key(&"USDC".to_string()) {
                let (v,p,x,y) = token_associated_pools.get(&"USDC".to_string()).unwrap();
                (true,v,p,x,y)
            } else {
                let (v,p,x,y) = token_associated_pools.get(&"ETH".to_string()).unwrap();
                (false,v,p,x,y)
            };
            let token_x_decimals: i32= self.db
                .query_decode("select decimals from tokens where address =  ? ",
                              vec![rbs::to_value!(token_x)])
                .await?;
            let token_y_decimals: i32= self.db
                .query_decode("select decimals from tokens where address =  ? ",
                              vec![rbs::to_value!(token_y)])
                .await?;
            db::calculate_price_hour(&self.db,pair_address.clone(),is_usdc,*vs_token0,token_x_decimals,token_y_decimals).await?
        };
        db::store_price(&mut self.db,token.address,price).await
    }
}

pub async fn run_tick_price(config: BackendConfig, db: rbatis::Rbatis) -> JoinHandle<()> {
    log::info!("Starting tick price!");
    let client = reqwest::Client::new();
    let base_url =  Url::from_str(&config.coingecko_url).unwrap();
    let task = TokenPriceTask::new(db,base_url,client);
    tokio::spawn(task.run_tick_price())
}

#[cfg(test)]
mod test {
    use std::time::Duration;
    use reqwest::Url;
    use std::str::FromStr;
    use std::collections::HashMap;

    #[tokio::test]
    pub async fn test_get_price_from_coingecko() {
        let base_url = Url::from_str(&"https://api.coingecko.com".to_string()).unwrap();
        let client = reqwest::Client::new();
        let coingecko_id = "weth".to_string();
        let simple_price_url = base_url
            .join(format!("api/v3/simple/price").as_str())
            .expect("Failed to join URL path");

        let simple_price = client
            .get(simple_price_url)
            .timeout(Duration::from_secs(120))
            .query(&[
                ("vs_currency", "usd"),
                ("ids", &coingecko_id),
            ])
            .send()
            .await
            .map_err(|err| anyhow::format_err!("CoinGecko API request failed: {}", err)).unwrap()
            .json::<HashMap<String,HashMap<String,f64>>>()
            .await
            .map_err(|err| anyhow::format_err!("Parse response data failed: {}", err)).unwrap();
        if simple_price.is_empty()  {
            println!("Response is empty");
        }
    }

}