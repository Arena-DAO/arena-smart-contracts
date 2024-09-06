use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

use crate::state::{ApplicationInfo, ApplicationStatus, ProjectLink, VestingConfiguration};

#[cw_serde]
pub struct InstantiateMsg {
    /// The DAO
    pub owner: String,
    pub config: VestingConfiguration,
}

#[cw_ownable_execute]
#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
pub enum ExecuteMsg {
    Apply(ApplyMsg),
    Update(Uint128, ApplyMsg),
    Withdraw {
        application_id: Uint128,
    },
    #[cw_orch(payable)]
    AcceptApplication {
        application_id: Uint128,
    },
    RejectApplication {
        application_id: Uint128,
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
    Application { application_id: Uint128 },
    #[returns(Vec<ApplicationResponse>)]
    Applications {
        start_after: Option<Uint128>,
        limit: Option<u32>,
        filter: Option<ApplicationsFilter>,
    },
    #[returns(cosmwasm_std::Addr)]
    PayrollAddress {},
}

#[cw_serde]
pub enum ApplicationsFilter {
    Status(ApplicationStatus),
    Applicant(String),
}

#[cw_serde]
pub struct ApplicationResponse {
    pub application_id: Uint128,
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
