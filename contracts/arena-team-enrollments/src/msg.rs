use arena_interface::competition::types::DaoConfig;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

/// Import shared types
use crate::state::{ApplicantStatus, EntryStatus};

/// Instantiate message (no fields now, but extensible)
#[cw_serde]
pub struct InstantiateMsg {}

/// Execute messages for the contract
#[cw_ownable::cw_ownable_execute]
#[derive(cw_orch::ExecuteFns)]
#[cw_serde]
pub enum ExecuteMsg {
    /// Creator creates a new team entry
    CreateEntry {
        title: String,
        description: String,
        category_id: Option<Uint128>,
        /// The standard dao config with an extra u64 field for taking in the cw4_group_code_id
        dao_config: DaoConfig<u64>,
    },

    /// Creator updates the status of a team entry
    UpdateEntryStatus { entry_id: u64, status: EntryStatus },

    /// Applicant applies to a specific entry
    Apply { entry_id: u64 },

    /// Withdraw application from a team entry
    WithdrawApplication { entry_id: u64 },

    /// Creator updates an applicant’s status (approve/reject)
    UpdateApplicantStatus {
        entry_id: u64,
        applicant: String,
        status: ApplicantStatus,
    },
}

/// Queries supported by the contract
#[cw_ownable::cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
pub enum QueryMsg {
    /// Get a specific team entry by ID
    #[returns(TeamEntryResponse)]
    GetEntry { entry_id: u64 },

    /// List all entries filtered by optional category_id and status
    #[returns(Vec<TeamEntryResponse>)]
    ListEntries {
        category_id: Option<Uint128>,
        status: Option<EntryStatus>,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Get the applicant status for a user in a given entry
    #[returns(ApplicantResponse)]
    GetApplicant { entry_id: u64, applicant: String },

    /// List applicants for a given entry filtered by status
    #[returns(Vec<ApplicantResponse>)]
    ListApplicants {
        entry_id: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

/// Response for a single team entry
#[cw_serde]
pub struct TeamEntryResponse {
    pub entry_id: u64,
    pub creator: Addr,
    pub title: String,
    pub description: String,
    pub category_id: Option<Uint128>,
    pub status: EntryStatus,
    pub created_at: u64,
}

/// Response for a single applicant
#[cw_serde]
pub struct ApplicantResponse {
    pub applicant: Addr,
    pub status: ApplicantStatus,
}
