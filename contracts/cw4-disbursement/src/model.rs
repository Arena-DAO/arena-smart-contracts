use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw4::Member;
use cw_disbursement::MemberShare;

#[cw_serde]
pub struct DumpStateResponse {
    pub members: Vec<Member>,
    pub total_weight: u64,
}

#[cw_serde]
pub struct GenericToken {
    pub addr: Option<Addr>,
    pub denom: Option<String>,
    pub amount: Uint128,
}

#[cw_serde]
pub struct InitialDisbursementData {
    pub key: String,
    pub shares: Vec<MemberShare>,
}
