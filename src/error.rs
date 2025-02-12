
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
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::ActixError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::CSVError(err) => {
                tracing::error!(%err);
                StatusCode::BAD_REQUEST
            }
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;
