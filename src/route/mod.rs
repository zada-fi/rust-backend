use crate::route::err::BackendError;
use serde::Serialize;

pub(crate) mod get_all_pools;
mod err;

#[derive(Debug, Serialize, Clone)]
pub struct BackendResponse<T: Clone + Serialize> {
    pub code: BackendError,
    pub error: Option<String>,
    pub data: Option<T>
}