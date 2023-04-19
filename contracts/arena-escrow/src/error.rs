use cosmwasm_std::{CheckedFromRatioError, OverflowError, StdError};
use cw_balance::BalanceError;
use cw_controllers::{AdminError, HookError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),

    #[error("{0}")]
    HookError(#[from] HookError),

    #[error("{0}")]
    AdminError(#[from] AdminError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("{0}")]
    BalanceError(#[from] BalanceError),

    #[error("Locked")]
    Locked {},

    #[error("NotFunded")]
    NotFunded {},

    #[error("NoneDue")]
    NoneDue {},
}
