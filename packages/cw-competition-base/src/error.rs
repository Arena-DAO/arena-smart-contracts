use cosmwasm_std::{
    CheckedFromRatioError, DecimalRangeExceeded, Instantiate2AddressError, OverflowError, StdError,
    Uint128,
};
use cw_competition::state::CompetitionStatus;
use cw_ownable::OwnershipError;
use cw_utils::ParseReplyError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum CompetitionError {
    #[error("{0}")]
    StdError(#[from] StdError),

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    OwnershipError(#[from] OwnershipError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("{0}")]
    Instantiate2AddressError(#[from] Instantiate2AddressError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("UnknownCompetitionId")]
    UnknownCompetitionId { id: u128 },

    #[error("CompetitionNotExpired")]
    CompetitionNotExpired {},

    #[error("UnknownEscrow")]
    UnknownEscrow { addr: String },

    #[error("UnknownReplyId")]
    UnknownReplyId { id: u64 },

    #[error("InvalidCompetitionStatus")]
    InvalidCompetitionStatus { current_status: CompetitionStatus },

    #[error("InvalidCategoryAndRulesets")]
    InvalidCategoryAndRulesets {
        category_id: Option<Uint128>,
        rulesets: Vec<Uint128>,
    },

    #[error("InvalidDistribution")]
    InvalidDistribution {},
}
