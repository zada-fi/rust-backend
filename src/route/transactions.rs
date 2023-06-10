use actix_web::{web, HttpResponse, HttpRequest};
use crate::server::AppState;
use crate::db;
use crate::route::{BackendResponse, DataWithPageCount};
use crate::route::err::BackendError;
use qstring::QString;
use crate::watcher::event::EventType;
use rbatis::rbdc::decimal::Decimal;
use std::str::FromStr;
use rbatis::rbdc::datetime::DateTime;
use crate::db::get_real_amount;
use crate::db_decimal_to_big;
use bigdecimal::BigDecimal;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct RespEventInfo {
    pub pair_name: String,
    pub pair_address: String,
    pub op_type: String,
    pub user_address: String,
    pub token_x_amount: String,
    pub token_y_amount: String,
    pub event_time: String,
    pub is_swap_x2y: Option<bool>,
}
pub async fn get_all_transactions(
    data: web::Data<AppState>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let rb = data.db.clone();
    let query_str = req.query_string();
    let qs = QString::from(query_str);
    let pg_no = qs.get("pg_no").unwrap_or("0").parse::<i32>().unwrap();
    let zero_decimal = Decimal::from_str("0").unwrap();
    match db::get_events_by_page_number(&rb,pg_no).await {
        Ok((page_count,txs)) => {
            let ret = txs.iter().map(|t| RespEventInfo {
                pair_name: format!("{}-{}",t.token_x_symbol,t.token_y_symbol),
                pair_address: t.pair_address.clone(),
                op_type: EventType::from_u8(t.event_type as u8).get_name(),
                user_address: t.from_account.clone().unwrap_or_default(),
                token_x_amount: get_real_amount(t.token_x_decimals as u8,db_decimal_to_big!(t.amount_x.clone().unwrap_or(zero_decimal.clone()).0)).0.to_string(),
                token_y_amount: get_real_amount(t.token_y_decimals as u8,db_decimal_to_big!(t.amount_y.clone().unwrap_or(zero_decimal.clone()).0)).0.to_string(),
                event_time: t.event_time.clone().unwrap_or(DateTime::from_timestamp(0)).0.to_string(),
                is_swap_x2y: t.is_swap_x2y,
            }).collect::<Vec<_>>();
            let resp = BackendResponse {
                code: BackendError::Ok,
                error: None,
                data: Some(DataWithPageCount {
                    page_count,
                    data: Some((page_count,ret))})
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