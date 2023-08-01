use crate::config::BackendConfig;
use actix_web::{HttpServer, web};
use std::net::SocketAddr;
use actix_web::App;
use crate::route::pools::get_all_pools;
use std::thread;
use crate::route::transactions::get_all_transactions;
use crate::route::pair_statistic_info::get_pair_statistic_info;
use crate::route::total_tvl::{get_total_tvl_by_day, get_total_volume_by_day};
use actix_cors::Cors;
use crate::route::launchpad::{get_all_projects, create_project, update_project, get_launchpad_stat_info, get_all_claimable_tokens, remove_project};

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: BackendConfig,
    pub db: rbatis::Rbatis,
}

pub(crate) async fn run_server(app_state: AppState) {
    thread::Builder::new()
        .spawn(move || {
            actix_rt::System::new().block_on(async move {
                run_rpc_server(app_state).await
            });
        })
        .expect("failed to start endpoint server");

}

pub async fn run_rpc_server(app_state: AppState) {
    let works_number = app_state.config.workers_number;
    let bind_to = SocketAddr::new("0.0.0.0".parse().unwrap(),
                                  app_state.config.server_port as u16);
    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(app_state.clone()))
            .route("/get_all_pools", web::get().to(get_all_pools))
            .route("/get_all_transactions", web::get().to(get_all_transactions))
            .route("/get_total_tvl_by_day", web::get().to(get_total_tvl_by_day))
            .route("/get_total_volume_by_day", web::get().to(get_total_volume_by_day))
            .route("/get_pair_statistic_info", web::get().to(get_pair_statistic_info))
            .route("/get_all_projects", web::get().to(get_all_projects))
            .route("/create_project", web::post().to(create_project))
            .route("/update_project", web::post().to(update_project))
            .route("/remove_project", web::post().to(remove_project))
            .route("/get_launchpad_stat_info", web::get().to(get_launchpad_stat_info))
            .route("/get_user_claimable_tokens",web::get().to(get_all_claimable_tokens))
    })
        .workers(works_number as usize)
        .bind(&bind_to)
        .expect("failed to bind")
        .run()
        .await
        .expect("failed to run endpoint server");
}