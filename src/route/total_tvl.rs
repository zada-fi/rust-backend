use actix_web::{web, HttpRequest, HttpResponse};
use crate::server::AppState;
use crate::db;
use crate::route::BackendResponse;
use crate::route::err::BackendError;
use rbatis::rbdc::decimal::Decimal;
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct RespTvlStatInfo {
    pub tvl_date: String,
    pub tvl_value: Decimal,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct RespVolumeStatInfo {
    pub volume_date: String,
    pub volume_value: Decimal,
}
pub async fn get_total_tvl_by_day(
    data: web::Data<AppState>,
    _req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let rb = data.db.clone();
    match db::get_all_tvls_by_day(&rb).await {
        Ok(pools) => {
            let ret = pools.iter().map(|p| RespTvlStatInfo {
                tvl_date: p.0.to_string(),
                tvl_value: p.1.clone()
            }).collect::<Vec<_>>();
            let resp = BackendResponse {
                code: BackendError::Ok,
                error: None,
                data: Some(ret)
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
            let ret = pools.iter().map(|p| RespVolumeStatInfo {
                volume_date: p.0.to_string(),
                volume_value: p.1.clone()
            }).collect::<Vec<_>>();
            let resp = BackendResponse {
                code: BackendError::Ok,
                error: None,
                data: Some(ret)
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