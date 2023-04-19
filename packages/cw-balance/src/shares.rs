use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, StdResult, Uint128};

#[cw_serde]
pub struct MemberShareVerified {
    pub addr: Addr,
    pub shares: Uint128,
}

impl MemberShare {
    pub fn to_verified(&self, deps: Deps) -> StdResult<MemberShareVerified> {
        Ok(MemberShareVerified {
            addr: deps.api.addr_validate(&self.addr)?,
            shares: self.shares,
        })
    }
}

#[cw_serde]
pub struct MemberShare {
    pub addr: String,
    pub shares: Uint128,
}

trait Validatable<T> {
    fn to_validated(&self, deps: Deps) -> StdResult<T>;
}
