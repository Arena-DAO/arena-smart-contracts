use cosmwasm_std::{CheckedFromRatioError, OverflowError, StdError};
use cw_balance::BalanceError;
use cw_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),

    #[error("{0}")]
    OwnershipError(#[from] OwnershipError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("{0}")]
    BalanceError(#[from] BalanceError),

    #[error("Locked")]
    Locked {},

    #[error("NotFullyFunded")]
    NotFullyFunded {},

    #[error("NoneDue")]
    NoneDue {},
}
