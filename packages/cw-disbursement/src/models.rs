use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw_tokens::GenericTokenBalance;

#[cw_serde]
pub struct MemberShare {
    pub addr: String,
    pub shares: Uint128,
}

#[cw_serde]
pub struct MemberBalance {
    pub member: String,
    pub balances: Vec<GenericTokenBalance>,
}

#[cw_serde]
pub struct DisbursementData {
    pub total_shares: Uint128,
    pub members: Vec<MemberShare>,
}

#[cw_serde]
pub struct DisbursementDataResponse {
    pub disbursement_data: Option<DisbursementData>,
}
