use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

use crate::state::{ApplicationInfo, ApplicationStatus, ProjectLink, VestingConfiguration};

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
    Update(ApplyMsg),
    Withdraw {},
    #[cw_orch(payable)]
    AcceptApplication {
        applicant: String,
    },
    RejectApplication {
        applicant: String,
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
    #[returns(ApplicationResponse)]
    Application { applicant: String },
    #[returns(Vec<ApplicationResponse>)]
    Applications {
        start_after: Option<String>,
        limit: Option<u32>,
        status: Option<ApplicationStatus>,
    },
}

#[cw_serde]
pub struct ApplicationResponse {
    pub applicant: String,
    pub application: ApplicationInfo,
}

#[cw_serde]
pub struct ApplyMsg {
    pub title: String,
    pub description: String,
    pub requested_amount: Uint128,
    pub project_links: Vec<ProjectLink>,
}

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}
