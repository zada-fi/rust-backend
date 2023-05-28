use actix_web::{web, HttpRequest, HttpResponse};
use crate::server::AppState;
use crate::db;
use crate::route::BackendResponse;
use crate::route::err::BackendError;

pub async fn get_total_tvl_by_day(
    data: web::Data<AppState>,
    _req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let rb = data.db.clone();
    match db::get_all_tvls_by_day(&rb).await {
        Ok(pools) => {
            let resp = BackendResponse {
                code: BackendError::Ok,
                error: None,
                data: Some(pools)
            };
            Ok(HttpResponse::Ok().json(resp))
        },
        Err(e) => {
            log::warn!("get_all_tvls_by_day from db failed,{:?}",e);
            let resp = BackendResponse {
                code: BackendError::DbErr,
                error: Some("get_all_tvls_by_day failed".to_string()),
                data: None::<()>,
            };
            Ok(HttpResponse::Ok().json(resp))
        }
    }

}

pub async fn get_total_volume_by_day(
    data: web::Data<AppState>,
    _req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let rb = data.db.clone();
    match db::get_all_volumes_by_day(&rb).await {
        Ok(pools) => {
            let resp = BackendResponse {
                code: BackendError::Ok,
                error: None,
                data: Some(pools)
            };
            Ok(HttpResponse::Ok().json(resp))
        },
        Err(e) => {
            log::warn!("get_all_volumes_by_day from db failed,{:?}",e);
            let resp = BackendResponse {
                code: BackendError::DbErr,
                error: Some("get_all_volumes_by_day failed".to_string()),
                data: None::<()>,
            };
            Ok(HttpResponse::Ok().json(resp))
        }
    }

}