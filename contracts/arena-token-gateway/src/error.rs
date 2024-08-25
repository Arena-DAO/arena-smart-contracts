use cosmwasm_std::{CheckedFromRatioError, DecimalRangeExceeded, OverflowError, StdError};
use cw_ownable::OwnershipError;
use cw_utils::{ParseReplyError, PaymentError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("{0}")]
    OwnershipError(#[from] OwnershipError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("Application already exists for this applicant")]
    ApplicationAlreadyExists {},

    #[error("Invalid application status")]
    InvalidApplicationStatus {},
}
