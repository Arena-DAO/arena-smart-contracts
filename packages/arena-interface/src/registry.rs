use cosmwasm_schema::{cw_serde, QueryResponses};
use cw_balance::Distribution;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
pub enum ExecuteMsg {
    SetDistribution { distribution: Distribution<String> },
    RemoveDistribution {},
}

#[cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
pub enum QueryMsg {
    #[returns(Option<Distribution<String>>)]
    GetDistribution { addr: String, height: Option<u64> },
}

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}
