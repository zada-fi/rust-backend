use crate::config::BackendConfig;
use actix_web::{HttpServer, web};
use std::net::SocketAddr;
use actix_web::App;
use crate::route::get_all_pools::get_all_pools;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: BackendConfig,
    pub db: rbatis::Rbatis,
}

pub(crate) async fn run_server(app_state: AppState) {
    let works_number = app_state.config.workers_number;
    let bind_to = SocketAddr::new("0.0.0.0".parse().unwrap(),
                                  app_state.config.server_port as u16);
    HttpServer::new(move || {
        // let mut cors = Cors::default();
        // if app_state.config.admin.enable_http_cors {
        //     cors = Cors::permissive();
        // }
        App::new()
            // .wrap(cors)
            .app_data(web::Data::new(app_state.clone()))
            .route("/get_all_pools", web::get().to(get_all_pools))
    })
        .workers(works_number as usize)
        .bind(&bind_to)
        .expect("failed to bind")
        .run()
        .await
        .expect("failed to run endpoint server");
}