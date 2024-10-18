use cosmwasm_std::{CheckedFromRatioError, CheckedMultiplyFractionError, OverflowError, StdError};
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

    #[error("{0}")]
    CheckedMultiplyFractionError(#[from] CheckedMultiplyFractionError),

    #[error("Cannot perform action while locked")]
    Locked {},

    #[error("Escrow is not fully funded")]
    NotFullyFunded {},

    #[error("The distribution is invalid: {msg}")]
    InvalidDistribution { msg: String },

    #[error("Invalid due: {msg}")]
    InvalidDue { msg: String },

    #[error("Cannot provide an empty balance")]
    EmptyBalance {},

    #[error("Already distributed")]
    AlreadyDistributed {},

    #[error("Unauthorized")]
    Unauthorized {},
}
