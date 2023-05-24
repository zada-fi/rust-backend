use actix_web::{web, HttpRequest, HttpResponse};
use crate::server::AppState;
use crate::db;
use crate::route::BackendResponse;
use crate::route::err::BackendError;

pub async fn get_total_tvl_volumes(
    data: web::Data<AppState>,
    _req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let rb = data.db.clone();
    match db::get_all_tvl_volumes(&rb).await {
        Ok(pools) => {
            let resp = BackendResponse {
                code: BackendError::Ok,
                error: None,
                data: Some(pools)
            };
            Ok(HttpResponse::Ok().json(resp))
        },
        Err(e) => {
            log::warn!("get_all_tvl_volumes from db failed,{:?}",e);
            let resp = BackendResponse {
                code: BackendError::DbErr,
                error: Some("get_all_tvl_volumes failed".to_string()),
                data: None::<()>,
            };
            Ok(HttpResponse::Ok().json(resp))
        }
    }

}