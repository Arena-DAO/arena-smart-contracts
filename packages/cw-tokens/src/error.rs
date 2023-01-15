use cosmwasm_std::{Addr, OverflowError, StdError};
use thiserror::Error;

use crate::GenericTokenType;

#[derive(Error, Debug, PartialEq)]
pub enum BalanceError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("MismatchedTypes")]
    MismatchedTypes {
        type1: GenericTokenType,
        type2: GenericTokenType,
    },

    #[error("MismatchedToken")]
    MismatchedToken {
        token1: (Option<Addr>, Option<String>),
        token2: (Option<Addr>, Option<String>),
    },

    #[error("TokenIdRequired")]
    TokenIdRequired {},
}
