use actix_web::{web, HttpResponse, HttpRequest};
use crate::server::AppState;
use crate::db;
use crate::route::{BackendResponse, DataWithPageCount};
use crate::route::err::BackendError;
use qstring::QString;

pub async fn get_all_transactions(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let rb = data.db.clone();
    let query_str = req.query_string();
    let qs = QString::from(query_str);
    let pg_no = qs.get("pg_no").unwrap_or("0").parse::<i32>().unwrap();
    match db::get_events_by_page_number(&rb,pg_no).await {
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