use actix_web::{http::StatusCode, ResponseError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database Error")]
    DbError(#[from] sea_orm::DbErr),
    #[error("Internal Error")]
    ActixError(#[from] actix_web::Error),
    #[error("CSV Error")]
    CSVError(#[from] csv::Error),
    
    #[error("{0}")]
    BadRequest(String),
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::ActixError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::CSVError(err) => {
                tracing::warn!(%err);
                StatusCode::BAD_REQUEST
            },
            Error::BadRequest(err) => {
                tracing::warn!(err);
                StatusCode::BAD_REQUEST
            },
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;

#[macro_export]
macro_rules! bail {
    ($kind:ident, $($arg:tt)*) => {
        return Err(crate::error::Error::$kind(format!($($arg)*)))
    }
}
