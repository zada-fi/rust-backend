use serde::Serialize;
use std::fmt::{Debug, Formatter};

#[derive(Copy, Clone, Serialize)]
pub enum BackendError {
    Ok = 0,
    DbErr = 100,
    InvalidParameters = 201,
    InternalErr = 500,
}

impl Debug for BackendError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "code: {}, message: {}", *self as i32, self.as_ref())
    }
}

impl AsRef<str> for BackendError {
    fn as_ref(&self) -> &'static str {
        match self {
            BackendError::Ok => "Ok",
            BackendError::DbErr => "Db error",
            BackendError::InvalidParameters => "Invalid request parameters",
            BackendError::InternalErr => "Server internal error",
        }
    }
}
