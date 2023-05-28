use actix_web::{web, HttpResponse, HttpRequest};
use crate::server::AppState;
use crate::db;
use crate::route::BackendResponse;
use crate::route::err::BackendError;
use qstring::QString;

pub async fn get_pair_statistic_info(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let rb = data.db.clone();
    let query_str = req.query_string();
    let qs = QString::from(query_str);
    let pg_no = qs.get("pg_no").unwrap_or("0").parse::<i32>().unwrap();
    match db::get_pools_stat_info_by_page_number(&rb,pg_no).await {
        Ok(pools) => {
            let resp = BackendResponse {
                code: BackendError::Ok,
                error: None,
                data: Some(pools)
            };
            Ok(HttpResponse::Ok().json(resp))
        },
        Err(e) => {
            log::warn!("get_pools_stat_info from db failed,{:?}",e);
            let resp = BackendResponse {
                code: BackendError::DbErr,
                error: Some("get_pools_stat_infofailed".to_string()),
                data: None::<()>,
            };
            Ok(HttpResponse::Ok().json(resp))
        }
    }

}