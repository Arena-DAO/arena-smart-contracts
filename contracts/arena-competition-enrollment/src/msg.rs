use arena_interface::fees::FeeInformation;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint128, Uint64};
use cw_utils::Expiration;
use dao_interface::state::ModuleInstantiateInfo;

use crate::state::{CompetitionType, EnrollmentEntryResponse};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
}

#[allow(clippy::large_enum_variant)]
#[cw_ownable::cw_ownable_execute]
#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
pub enum ExecuteMsg {
    #[cw_orch(payable)]
    CreateEnrollment {
        /// Override the minimum members for the competition
        min_members: Option<Uint64>,
        max_members: Uint64,
        /// The entry fee of the competition
        entry_fee: Option<Coin>,
        expiration: Expiration,
        category_id: Option<Uint128>,
        competition_info: CompetitionInfoMsg,
        competition_type: CompetitionType,
        group_contract_info: ModuleInstantiateInfo,
    },
    TriggerExpiration {
        id: Uint128,
        escrow_id: u64,
    },
    #[cw_orch(payable)]
    Enroll {
        id: Uint128,
    },
    Withdraw {
        id: Uint128,
    },
}

#[cw_serde]
pub struct CompetitionInfoMsg {
    pub name: String,
    pub description: String,
    pub expiration: Expiration,
    pub rules: Option<Vec<String>>,
    pub rulesets: Option<Vec<Uint128>>,
    pub banner: Option<String>,
    pub additional_layered_fees: Option<Vec<FeeInformation<String>>>,
}

#[cw_serde]
pub enum EnrollmentFilter {
    Category { category_id: Option<Uint128> },
    Host(String),
}

#[cw_ownable::cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
pub enum QueryMsg {
    #[returns(Vec<EnrollmentEntryResponse>)]
    Enrollments {
        start_after: Option<Uint128>,
        limit: Option<u32>,
        filter: Option<EnrollmentFilter>,
    },
    #[returns(EnrollmentEntryResponse)]
    Enrollment { enrollment_id: Uint128 },
    #[returns(Uint128)]
    EnrollmentCount {},
}

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}

#[cw_serde]
pub struct SudoMsg {
    pub enrollment_entry_response: EnrollmentEntryResponse,
}
