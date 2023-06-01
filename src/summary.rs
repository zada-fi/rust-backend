
use crate::db;
use std::time::Duration;
use bigdecimal::BigDecimal;
use std::str::FromStr;
use rbatis::rbdc::decimal::Decimal;
use crate::config::BackendConfig;
use tokio::task::JoinHandle;
use crate::db::tables::EventStat;
use rbatis::rbdc::date::Date;
use crate::db_decimal_to_big;
use num::BigUint;

pub struct TickSummaryTask {
    pub db: rbatis::Rbatis,
    pub config: BackendConfig,
}
impl TickSummaryTask {
    pub fn new(db:rbatis::Rbatis,config:BackendConfig)->Self {
        Self {
            db,
            config
        }
    }
    pub async fn run_tick_summary(mut self) {
        println!("run_tick_summary");
        let mut tx_poll = tokio::time::interval(Duration::from_secs(3600));
        loop {
            tx_poll.tick().await;
            if let Err(e) = self.statistic_summary().await {
                println!("statistic_summary failed {:?}",e);
            }
        }
    }
    pub async fn statistic_summary(&mut self) ->anyhow::Result<()> {
        let days = db::get_unstated_days(&self.db,&self.config.stat_start_date).await?;
        for day in days {
            let tvls = db::get_pools_day_tvl(&self.db, day.clone()).await?;
            let volumes = db::get_pools_day_volume(&self.db, day.clone()).await?;
            let mut stats = Vec::new();
            for (tvl,volume) in tvls.iter().zip(volumes) {
                let (price,x_price) = db::get_pool_usd_price(&self.db,tvl.pair_address.clone()).await.unwrap_or_default();
                let (x_decimals,y_decimals) = db::get_token_decimals_in_pool(&self.db,tvl.pair_address.clone()).await?;
                let x_pow_decimals = BigDecimal::from_str(&BigUint::from(10u32).pow(x_decimals as u32).to_string()).unwrap();
                let y_pow_decimals = BigDecimal::from_str(&BigUint::from(10u32).pow(y_decimals as u32).to_string()).unwrap();
                let (usd_tvl,usd_volume) = if x_price {
                    (price.clone() * db_decimal_to_big!(tvl.amount_x.0) / x_pow_decimals.clone(),
                     price.clone() * db_decimal_to_big!(volume.amount_x.0) / x_pow_decimals)
                } else {
                    (price.clone() * db_decimal_to_big!(tvl.amount_y.0) / y_pow_decimals.clone(),
                     price.clone() * db_decimal_to_big!(volume.amount_y.0) / y_pow_decimals)
                };

                println!("pool {:?} tvl : {:?},volume: {:?}",tvl.pair_address.clone(),usd_tvl,usd_volume);
                let stat = EventStat {
                    pair_address: tvl.pair_address.clone(),
                    stat_date: Date::from_str(&tvl.day).unwrap(),
                    x_reserves: tvl.amount_x.clone(),
                    y_reserves: tvl.amount_y.clone(),
                    x_volume: volume.amount_x,
                    y_volume: volume.amount_y,
                    usd_tvl:Decimal::from_str(&format!("{:.18}",usd_tvl)).unwrap(),
                    usd_volume: Decimal::from_str(&format!("{:.18}",usd_volume)).unwrap(),
                };
                stats.push(stat);

            }
            db::save_day_stats(&mut self.db, stats).await?;
        }
        Ok(())
    }
}

pub async fn run_tick_summary(db: rbatis::Rbatis,config:BackendConfig) -> JoinHandle<()> {
    log::info!("Starting tick summary!");
    let task = TickSummaryTask::new(db,config);
    tokio::spawn(task.run_tick_summary())
}


#[cfg(test)]
mod test {
    use super::*;
    use rbatis::Rbatis;

    #[tokio::test]
    async fn statistic_summary() {
        let rb = Rbatis::new();
        let db_url = "postgres://postgres:postgres123@localhost/backend";
        rb.init(rbdc_pg::driver::PgDriver {}, db_url).unwrap();
        let pool = rb
            .get_pool()
            .expect("get pool failed");
        pool.resize(2);
        let config = BackendConfig {
            server_port: 0,
            database_url: "".to_string(),
            coingecko_url: "".to_string(),
            db_pool_size: 0,
            remote_web3_url: "".to_string(),
            watch_time_interval: 0,
            tick_price_time_interval: 0,
            workers_number: 0,
            contract_address: Default::default(),
            sync_start_block: 0,
            stat_start_date: "2023-03-28".to_string()
        };
        let mut task = TickSummaryTask::new(rb,config);
        task.statistic_summary().await.unwrap();
    }
}