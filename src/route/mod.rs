use crate::route::err::BackendError;
use serde::{Serialize,Deserialize};

pub(crate) mod pools;
mod err;
pub mod transactions;

#[derive(Debug, Serialize, Clone,Deserialize)]
pub struct BackendCommonReq {
    pub pg_no: i32,
}
#[derive(Debug, Serialize, Clone)]
pub struct BackendResponse<T: Clone + Serialize> {
    pub code: BackendError,
    pub error: Option<String>,
    pub data: Option<T>
}