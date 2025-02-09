use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

use crate::competition::types::EnrollmentEntryResponse;

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
    #[returns(bool)]
    IsMember {
        enrollment_id: Uint128,
        addr: String,
    },
}

#[cw_serde]
pub enum EnrollmentFilter {
    Category { category_id: Option<Uint128> },
    Host(String),
}
