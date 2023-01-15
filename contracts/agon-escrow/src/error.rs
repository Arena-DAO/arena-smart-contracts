use cosmwasm_std::StdError;
use cw_controllers::HookError;
use cw_disbursement::DisbursementError;
use cw_tokens::BalanceError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),

    #[error("{0}")]
    Hook(#[from] HookError),

    #[error("{0}")]
    Balance(#[from] BalanceError),

    #[error("{0}")]
    Disbursement(#[from] DisbursementError),

    #[error("InvalidState")]
    InvalidState {},

    #[error("Unauthorized")]
    Unauthorized {},
}
