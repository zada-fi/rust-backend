use actix_web::{web, HttpResponse};
use crate::server::AppState;
use crate::db;
use crate::route::{BackendResponse, BackendCommonReq, DataWithPageCount};
use crate::route::err::BackendError;

pub async fn get_all_transactions(
    data: web::Data<AppState>,
    msg: web::Json<BackendCommonReq>,
) -> actix_web::Result<HttpResponse> {
    let rb = data.db.clone();

    match db::get_events_by_page_number(&rb,msg.pg_no).await {
        Ok((page_count,txs)) => {
            let resp = BackendResponse {
                code: BackendError::Ok,
                error: None,
                data: Some(DataWithPageCount {
                    page_count,
                    data: Some(txs)})
            };
            Ok(HttpResponse::Ok().json(resp))
        },
        Err(e) => {
            log::warn!("get_all_transactions from db failed,{:?}",e);
            let resp = BackendResponse {
                code: BackendError::DbErr,
                error: Some("get events failed".to_string()),
                data: None::<()>,
            };
            Ok(HttpResponse::Ok().json(resp))
        }
    }

}