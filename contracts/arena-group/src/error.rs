use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),

    #[error("{0}")]
    OwnershipError(#[from] OwnershipError),

    #[error("Unauthorized")]
    Unauthorized {},
    #[error("No members provided")]
    NoMembers {},
    #[error("Cannot add duplicates {member}")]
    DuplicateMembers { member: cosmwasm_std::Addr },
    #[error("User is not a member {member}")]
    NotMember { member: cosmwasm_std::Addr },
}
