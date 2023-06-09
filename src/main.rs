use dotenvy::dotenv;
use crate::config::BackendConfig;
use rbatis::Rbatis;
use crate::server::AppState;
use rbatis::rbdc::rt::block_on;
use std::cell::RefCell;
use futures::channel::mpsc;
use futures::SinkExt;
use futures::StreamExt;
use crate::watcher::watch::run_watcher;

pub mod config;
pub mod watcher;
pub mod server;
pub mod db;
pub mod route;

/// make an Rbatis
pub fn init_db(db_url:String,pool_size: usize) -> Rbatis {
    let rb = Rbatis::new();
    rb.init(rbdc_pg::driver::PgDriver {}, &db_url).unwrap();
    let pool = rb
        .get_pool()
        .expect("get pool failed");
    pool.resize(pool_size);
    log::info!("postgres database init ok!");
    return rb;
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().expect("Config file not found");
    let config = BackendConfig::from_env();
    let db = init_db(config.database_url.clone(), config.db_pool_size as usize);
    let app_state = AppState {
        config:config.clone(),
        db: db.clone()
    };
    server::run_server(app_state).await;
    let watcher_handlers = run_watcher(config,db).await;

    // handle ctrl+c
    let (stop_signal_sender, mut stop_signal_receiver) = mpsc::channel(256);
    {
        let stop_signal_sender = RefCell::new(stop_signal_sender.clone());
        ctrlc::set_handler(move || {
            let mut sender = stop_signal_sender.borrow_mut();
            block_on(sender.send(true)).expect("Ctrl+C signal send");
        })
            .expect("Error setting Ctrl+C handler");
    }

    tokio::select! {
        Err(e) = watcher_handlers => {
            if e.is_panic() { log::error!("The one of watcher actors unexpectedly panic:{}", e) }
            log::error!("Watchers actors aren't supposed to finish any of their execution")
        },
        _ = async { stop_signal_receiver.next().await } => {
            log::warn!("Stop signal received, shutting down");
        }
    };

    Ok(())
}
