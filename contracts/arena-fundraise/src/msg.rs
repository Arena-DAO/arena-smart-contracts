use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint128};
use cw_utils::{Duration, Expiration};

use crate::state::Config;

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}

#[cw_serde]
pub struct InstantiateMsg {
    pub fundraise: Coin,
    pub deposit_denom: String,
    pub soft_cap: Uint128,
    pub hard_cap: Option<Uint128>,
    pub start: Option<Expiration>,
    pub duration: Duration,
}

#[cw_serde]
pub enum ExecuteMsg {
    Deposit {},
    Withdraw {},
    Expire {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},
    #[returns(Uint128)]
    TotalDeposited {},
    #[returns(Option<Uint128>)]
    Deposit { addr: String },
    #[returns(Option<Uint128>)]
    Reward { addr: String },
    #[returns(DumpStateResponse)]
    DumpState { addr: Option<String> },
}

#[cw_serde]
pub struct DumpStateResponse {
    pub config: Config,
    pub deposit: Option<Uint128>,
    pub reward: Option<Uint128>,
    pub total_deposited: Uint128,
    pub has_expired: bool,
    pub has_started: Option<bool>,
}
