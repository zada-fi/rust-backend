use actix_web::{web, HttpResponse};
use crate::server::AppState;
use crate::db;
use crate::route::{BackendResponse, BackendCommonReq};
use crate::route::err::BackendError;

pub async fn get_all_pools(
    data: web::Data<AppState>,
    msg: web::Json<BackendCommonReq>,
) -> actix_web::Result<HttpResponse> {
    let rb = data.db.clone();

    match db::get_pools_by_page_number(&rb,msg.pg_no).await {
        Ok(pools) => {
            let resp = BackendResponse {
                code: BackendError::Ok,
                error: None,
                data: Some(pools)
            };
            Ok(HttpResponse::Ok().json(resp))
        },
        Err(e) => {
            log::warn!("get_all_pools from db failed,{:?}",e);
            let resp = BackendResponse {
                code: BackendError::DbErr,
                error: Some("get pools failed".to_string()),
                data: None::<()>,
            };
            Ok(HttpResponse::Ok().json(resp))
        }
    }

}