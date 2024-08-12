use cosmwasm_schema::{cw_serde, QueryResponses};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

use crate::state::VestingConfiguration;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub config: VestingConfiguration,
}

#[cw_ownable_execute]
#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
pub enum ExecuteMsg {
    Apply(ApplyMsg),
    Accept {
        application_id: u64,
    },
    Reject {
        application_id: u64,
        reason: Option<String>,
    },
    UpdateVestingConfiguration {
        config: VestingConfiguration,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
pub enum QueryMsg {
    #[returns(VestingConfiguration)]
    VestingConfiguration {},
}

#[cw_serde]
pub struct ApplyMsg {
    pub applicant: String,
    pub title: String,
    pub description: Option<String>,
    pub project_info: Option<String>,
    pub team_info: Option<String>,
}
