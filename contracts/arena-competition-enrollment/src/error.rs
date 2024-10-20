use cosmwasm_std::{
    CheckedFromRatioError, DecimalRangeExceeded, Instantiate2AddressError, OverflowError, StdError,
    Uint128, Uint64,
};
use cw_ownable::OwnershipError;
use cw_utils::{Expiration, ParseReplyError, PaymentError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),

    #[error("{0}")]
    OwnershipError(#[from] OwnershipError),

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("{0}")]
    Instantiate2AddressError(#[from] Instantiate2AddressError),

    #[error("UnknownReplyId")]
    UnknownReplyId { id: u64 },

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Already enrolled")]
    AlreadyEnrolled {},

    #[error("Cannot trigger creation with {current_members} members")]
    TriggerFailed {
        max_members: Uint64,
        current_members: Uint64,
        expiration: Expiration,
    },

    #[error("Competition has already been generated or expired")]
    AlreadyExpired {},

    #[error("Entry fee was not paid")]
    EntryFeeNotPaid { fee: Uint128 },

    #[error("Not enrolled")]
    NotEnrolled {},

    #[error("Enrollment is at max members already")]
    EnrollmentMaxMembers {},
}
