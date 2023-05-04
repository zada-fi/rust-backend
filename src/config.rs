use std::env;
use serde::Deserialize;
use web3::types::H160;

#[derive(Default, Debug, Deserialize, Clone)]
pub struct BackendConfig {
    pub server_port:u16,
    pub database_url: String,
    pub db_pool_size: u16,
    pub remote_web3_url: String,
    pub watch_time_interval: u32,
    pub workers_number: u16,
    pub contract_address: H160,
}

impl BackendConfig {
    pub fn from_env() -> Self {
        let server_port = env::var("SERVER_PORT").unwrap_or_default()
            .parse::<u16>().unwrap_or(8088u16);
        let database_url = env::var("DATABASE_URL").unwrap_or_default();
        let remote_web3_url = env::var("REMOTE_WEB3_URL").unwrap_or_default();
        let watch_time_interval = env::var("WATCH_TIME_INTERVAL").unwrap_or_default()
            .parse::<u32>().unwrap_or(60u32);
        let workers_number = env::var("WORKERS_NUMBER").unwrap_or_default()
            .parse::<u16>().unwrap_or(1u16);
        let db_pool_size = env::var("DB_POOL_SIZE").unwrap_or_default()
            .parse::<u16>().unwrap_or(1u16);
        let contract_address = env::var("CONTRACT_ADDRESS").unwrap_or_default();
        Self {
            server_port,
            database_url,
            remote_web3_url,
            watch_time_interval,
            workers_number,
            db_pool_size,
            contract_address: H160::from_slice(&hex::decode(contract_address).unwrap())
        }
    }
}