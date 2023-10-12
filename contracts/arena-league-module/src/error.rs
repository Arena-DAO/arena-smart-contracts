use cosmwasm_std::StdError;
use cw_competition_base::error::CompetitionError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),

    #[error("{0}")]
    CompetitionError(#[from] CompetitionError),

    #[error("CompetitionModuleNotAvailable")]
    CompetitionModuleNotAvailable { key: String },
}
