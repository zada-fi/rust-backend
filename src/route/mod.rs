use crate::route::err::BackendError;
use serde::{Serialize,Deserialize};

pub(crate) mod pools;
mod err;
pub mod transactions;
pub mod pair_statistic_info;
pub mod total_tvl;
pub mod launchpad;

#[derive(Debug, Serialize, Clone,Deserialize)]
pub struct BackendCommonReq {
    pub pg_no: i32,
}
#[derive(Debug, Serialize, Clone)]
pub struct DataWithPageCount<T: Clone + Serialize> {
    pub page_count: usize,
    pub data: Option<T>
}
#[derive(Debug, Serialize, Clone)]
pub struct BackendResponse<T: Clone + Serialize> {
    pub code: BackendError,
    pub error: Option<String>,
    pub data: Option<T>
}


// #[macro_export]
// macro_rules! update_project_column {
//     ($rb:expr,$pj_name:expr, $column_name:expr, $column_value:expr) => {{
//             $rb.exec(format!("update projects set $column_name = ? \
//      where project_name = ?",
//             vec![rbs::to_value!($column_value)])
//         .await?;
//     }};
// }