use std::fmt::format;
use actix_web::{web, HttpResponse, HttpRequest};
use crate::server::AppState;
use crate::db;
use crate::route::BackendResponse;
use crate::route::err::BackendError;
use qstring::QString;
use rbatis::rbdc::decimal::Decimal;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RespPoolInfo {
    pub pair_name: String,
    pub pair_address: String,
    pub token_x_address: String,
    pub token_y_address: String,
    pub x_reserves: Decimal,
    pub y_reserves: Decimal,
    pub apy: String,
}
pub async fn get_all_pools(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let rb = data.db.clone();
    let query_str = req.query_string();
    let qs = QString::from(query_str);
    let pg_no = qs.get("pg_no").unwrap_or("0").parse::<i32>().unwrap();
    match db::get_pools_by_page_number(&rb,pg_no).await {
        Ok(pools) => {
            let pools_address = pools.1.iter().map(|p| p.pair_address.clone()).collect::<Vec<_>>();
            let ret_apy = db::get_pools_apy(&rb,pools_address).await;
            if let Err(e) = ret_apy {
                log::warn!("get_pools_apy from db failed,{:?}",e);
                let resp = BackendResponse {
                    code: BackendError::DbErr,
                    error: Some("get get_pools_apy failed".to_string()),
                    data: None::<()>,
                };
                Ok(HttpResponse::Ok().json(resp))
            } else {
                let pools_apy = ret_apy.unwrap();
                let resp_pools = pools.1.iter().zip(pools_apy).map(|(p,a)| {
                    RespPoolInfo {
                        pair_name: format!("{}-{}",p.token_x_symbol.clone(),p.token_y_symbol.clone()),
                        pair_address: p.pair_address.clone(),
                        token_x_address: p.token_x_address.clone(),
                        token_y_address: p.token_y_address.clone(),
                        x_reserves: p.token_x_reserves.clone(),
                        y_reserves: p.token_y_reserves.clone(),
                        apy: format!("{:.2}",a),
                    }
                }).collect::<Vec<_>>();
                let resp = BackendResponse {
                    code: BackendError::Ok,
                    error: None,
                    data: Some((pools.0,resp_pools))
                };
                Ok(HttpResponse::Ok().json(resp))
            }
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