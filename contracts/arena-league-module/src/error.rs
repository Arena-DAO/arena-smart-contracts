use cosmwasm_std::{OverflowError, StdError};
use cw_competition_base::error::CompetitionError;
use cw_ownable::OwnershipError;
use cw_utils::Expiration;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),

    #[error("{0}")]
    CompetitionError(#[from] CompetitionError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    OwnershipError(#[from] OwnershipError),

    #[error("InvalidExecute")]
    InvalidExecute,

    #[error("NotExpired")]
    NotExpired { expiration: Expiration },
}
