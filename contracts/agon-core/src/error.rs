use std::num::ParseIntError;

use cosmwasm_std::{CheckedFromRatioError, DecimalRangeExceeded, OverflowError, StdError};
use cw_utils::ParseReplyError;
use dao_pre_propose_base::error::PreProposeError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("{0}")]
    ParseIntError(#[from] ParseIntError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("UnknownReplyId")]
    UnknownReplyId { id: u64 },

    #[error("{0}")]
    PrePropose(#[from] PreProposeError),

    #[error("Unauthorized")]
    Unauthorized {},
}
