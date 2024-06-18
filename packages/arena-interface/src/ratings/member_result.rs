use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};
use cw_address_like::AddressLike;

#[cw_serde]
pub struct MemberResult<T: AddressLike> {
    pub addr: T,
    pub result: Decimal,
}

impl From<MemberResult<Addr>> for MemberResult<String> {
    fn from(member: MemberResult<Addr>) -> Self {
        MemberResult {
            addr: member.addr.to_string(),
            result: member.result,
        }
    }
}
