use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, StdResult, Uint128};

#[cw_serde]
pub struct MemberShareValidated {
    pub addr: Addr,
    pub shares: Uint128,
}

impl MemberShare {
    pub fn to_validated(&self, deps: Deps) -> StdResult<MemberShareValidated> {
        Ok(MemberShareValidated {
            addr: deps.api.addr_validate(&self.addr)?,
            shares: self.shares,
        })
    }
}

pub struct MemberShare {
    pub addr: String,
    pub shares: Uint128,
}
