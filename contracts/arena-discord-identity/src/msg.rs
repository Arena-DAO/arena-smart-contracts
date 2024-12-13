use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint64};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub faucet_amount: Coin,
}

#[cw_ownable::cw_ownable_execute]
#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
pub enum ExecuteMsg {
    SetProfile { addr: String, user_id: Uint64 },
    SetFaucetAmount { amount: Coin },
    Withdraw {},
}

#[cw_ownable::cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
pub enum QueryMsg {
    #[returns(Option<Uint64>)]
    UserId { addr: String },
}

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}
