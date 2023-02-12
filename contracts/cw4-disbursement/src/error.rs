use cosmwasm_std::{DivideByZeroError, OverflowError, StdError};
use cw4_group::ContractError as Cw4GroupError;
use cw_controllers::{AdminError, HookError};
use cw_disbursement::DisbursementError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Hook(#[from] HookError),

    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    DivideByZero(#[from] DivideByZeroError),

    #[error("{0}")]
    Cw4Group(#[from] Cw4GroupError),

    #[error("{0}")]
    Disbursement(#[from] DisbursementError),

    #[error("MemberNotFound")]
    MemberNotFound {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("InvalidToken")]
    InvalidToken {},

    #[error("ZeroSharesTotal")]
    ZeroSharesTotal {},
}
