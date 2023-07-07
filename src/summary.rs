
use crate::db;
use std::time::Duration;
use bigdecimal::BigDecimal;
use std::str::FromStr;
use rbatis::rbdc::decimal::Decimal;
use crate::config::BackendConfig;
use tokio::task::JoinHandle;
use crate::db::tables::{EventStatData, TvlStat, VolumeStat, HistoryStatInfo};
use rbatis::rbdc::date::Date;
use crate::db_decimal_to_big;
use num::BigUint;
use std::ops::Add;
use std::collections::HashMap;

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
    pub async fn calc_usd_amount(&self,data:&EventStatData)->anyhow::Result<BigDecimal> {
        let (price,x_price) = db::get_pool_usd_price(
            &self.db,data.pair_address.clone()).await?;
        let (x_decimals,y_decimals) = db::get_token_decimals_in_pool(
            &self.db,data.pair_address.clone()).await?;
        let x_pow_decimals = BigDecimal::from_str(&BigUint::from(10u32).pow(x_decimals as u32).to_string()).unwrap();
        let y_pow_decimals = BigDecimal::from_str(&BigUint::from(10u32).pow(y_decimals as u32).to_string()).unwrap();
        let usd_amount = if x_price {
            price.clone() * db_decimal_to_big!(data.amount_x.0) / x_pow_decimals.clone()
        } else {
            price.clone() * db_decimal_to_big!(data.amount_y.0) / y_pow_decimals.clone()
        };
        Ok(usd_amount)
    }
    pub async fn statistic_summary(&mut self) ->anyhow::Result<()> {
        let days = db::get_unstated_days(&self.db,&self.config.stat_start_date).await?;
        for day in days {
            let tvls = db::get_pools_day_tvl(&self.db, day.clone()).await?;
            let volumes = db::get_pools_day_volume(&self.db, day.clone()).await?;
            let mut tvl_stats = Vec::new();
            let mut volume_stats = Vec::new();
            //update tvl on day
            let pre_tvls = db::get_pools_pre_day_tvl(&self.db,day.clone()).await?;
            let mut total_volume_by_day = BigDecimal::from(0);
            let mut total_tvl_by_day = BigDecimal::from(0);
            for tvl in tvls.iter(){
                let usd_amount = self.calc_usd_amount(tvl).await?;
                println!("pool {:?} tvl : {:?} on day {:?}",tvl.pair_address.clone(),usd_amount,day);
                let stat = TvlStat {
                    pair_address: tvl.pair_address.clone(),
                    stat_date: Date::from_str(&tvl.day).unwrap(),
                    x_reserves: tvl.amount_x.clone(),
                    y_reserves: tvl.amount_y.clone(),
                    usd_tvl:Decimal::from_str(&format!("{:.18}", usd_amount)).unwrap(),
                };
                tvl_stats.push(stat);
                total_tvl_by_day += usd_amount;
            }

            for tvl in pre_tvls.iter() {
                if let Some(new_tvl) = tvl_stats.iter().find(|s| s.pair_address == tvl.pair_address) {
                    continue;
                } else {
                    total_tvl_by_day += BigDecimal::from_str(&tvl.usd_tvl.0.to_string()).unwrap();
                }
            }

            for volume in volumes.iter(){
                let usd_amount = self.calc_usd_amount(volume).await?;
                println!("pool {:?} volume : {:?} on day {:?}",volume.pair_address.clone(),usd_amount,day);
                let stat = VolumeStat {
                    pair_address: volume.pair_address.clone(),
                    stat_date: Date::from_str(&volume.day).unwrap(),
                    x_volume: volume.amount_x.clone(),
                    y_volume: volume.amount_y.clone(),
                    usd_volume: Decimal::from_str(&format!("{:.18}", usd_amount)).unwrap(),
                };
                volume_stats.push(stat);
                total_volume_by_day += usd_amount;

            }

            let history_stat = HistoryStatInfo {
                stat_date: Date::from_str(&day).unwrap(),
                usd_tvl: Decimal::from_str(&total_tvl_by_day.to_string()).unwrap(),
                usd_volume: Decimal::from_str(&total_volume_by_day.to_string()).unwrap(),
            };
            db::save_day_tvl_stats(&mut self.db, tvl_stats).await?;
            db::save_day_volume_stats(&mut self.db, volume_stats).await?;
            // todo:should save history stats on batch
            db::save_history_stat(&mut self.db,history_stat).await?;
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
            launchpad_address: Default::default(),
            sync_start_block: 0,
            stat_start_date: "2023-03-28".to_string()
        };
        let mut task = TickSummaryTask::new(rb,config);
        task.statistic_summary().await.unwrap();
    }
}