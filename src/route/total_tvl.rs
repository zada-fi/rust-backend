use actix_web::{web, HttpRequest, HttpResponse};
use crate::server::AppState;
use crate::db;
use crate::route::BackendResponse;
use crate::route::err::BackendError;
use rbatis::rbdc::decimal::Decimal;
use bigdecimal::BigDecimal;
use std::str::FromStr;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct RespTvlStatInfo {
    pub tvl_date: String,
    pub tvl_value: Decimal,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct InnerRespVolumeStatInfo {
    pub volume_date: String,
    pub volume_value: Decimal,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct RespVolumeStatInfo {
    pub total_volume: Decimal,
    pub stat_infos: Vec<InnerRespVolumeStatInfo>,
}
pub async fn get_total_tvl_by_day(
    data: web::Data<AppState>,
    _req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let rb = data.db.clone();
    match db::get_all_tvls_by_day(&rb).await {
        Ok(pools) => {
            let ret = pools.iter().map(|p| RespTvlStatInfo {
                tvl_date: p.0.clone(),
                tvl_value: Decimal::from_str(&format!("{:.2}",BigDecimal::from_str(&p.1.0.to_string()).unwrap())).unwrap()
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
            let mut stat_infos = Vec::new();
            let mut total_volume = BigDecimal::from(0);
            for p in pools {
                let day_volume = BigDecimal::from_str(&p.1.0.to_string()).unwrap();
                total_volume += day_volume.clone();
                let day_volume_info = InnerRespVolumeStatInfo {
                    volume_date: p.0.clone(),
                    volume_value: Decimal::from_str(&format!("{:.2}", day_volume)).unwrap()
                };
                stat_infos.push(day_volume_info);
            }
            let ret = RespVolumeStatInfo {
                total_volume: Decimal::from_str(&format!("{:.2}",total_volume)).unwrap(),
                stat_infos,
            };
            let resp = BackendResponse {
                code: BackendError::Ok,
                error: None,
                data: Some(ret)
            };
            Ok(HttpResponse::Ok().json(resp))
        },
        Err(e) => {
            log::error!("get_all_volumes_by_day from db failed,{:?}",e);
            let resp = BackendResponse {
                code: BackendError::DbErr,
                error: Some("get_all_volumes_by_day failed".to_string()),
                data: None::<()>,
            };
            Ok(HttpResponse::Ok().json(resp))
        }
    }

}