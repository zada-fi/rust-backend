use actix_web::{web, HttpResponse, HttpRequest};
use crate::server::AppState;
use crate::db;
use crate::route::BackendResponse;
use crate::route::err::BackendError;
use qstring::QString;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PairStatInfo {
    pub pair_name: String,
    pub pair_address: String,
    pub usd_volume: String,
    pub usd_volume_week: String,
    pub usd_tvl: String
}
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
            let ret = pools.1.iter().map(|p| PairStatInfo {
                pair_name: format!("{}-{}",p.token_x_symbol,p.token_y_symbol),
                pair_address: p.pair_address.clone(),
                usd_volume: p.usd_volume.0.to_string(),
                usd_volume_week: p.usd_volume_week.0.to_string(),
                usd_tvl: p.usd_tvl.0.to_string()
            }).collect::<Vec<_>>();
            let resp = BackendResponse {
                code: BackendError::Ok,
                error: None,
                data: Some((pools.0,ret))
            };
            Ok(HttpResponse::Ok().json(resp))
        },
        Err(e) => {
            println!("get_pools_stat_info from db failed,{:?}",e);
            let resp = BackendResponse {
                code: BackendError::DbErr,
                error: Some("get_pools_stat_info failed".to_string()),
                data: None::<()>,
            };
            Ok(HttpResponse::Ok().json(resp))
        }
    }

}