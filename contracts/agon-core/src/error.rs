use std::num::ParseIntError;

use cosmwasm_std::{CheckedFromRatioError, DecimalRangeExceeded, OverflowError, StdError};
use cw_utils::ParseReplyError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    ParseReply(#[from] ParseReplyError),

    #[error("{0}")]
    ParseInt(#[from] ParseIntError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    DecimalExceeded(#[from] DecimalRangeExceeded),

    #[error("{0}")]
    CheckedFromRatio(#[from] CheckedFromRatioError),

    #[error("InvalidWagerStatus")]
    InvalidWagerStatus {},

    #[error("UnknownReplyId")]
    UnknownReplyId { id: u64 },

    #[error("UnknownWagerId")]
    UnknownWagerId { id: u128 },

    #[error("NoProposalMultiple")]
    NoProposalMultiple {},

    #[error("Unauthorized")]
    Unauthorized {},
}
