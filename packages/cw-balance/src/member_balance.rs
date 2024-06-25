use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, StdResult};

use crate::{BalanceUnchecked, BalanceVerified};

#[cw_serde]
pub struct MemberBalanceChecked {
    pub addr: Addr,
    pub balance: BalanceVerified,
}

#[cw_serde]
pub struct MemberBalanceUnchecked {
    pub addr: String,
    pub balance: BalanceUnchecked,
}

impl MemberBalanceUnchecked {
    pub fn into_checked(self, deps: Deps) -> StdResult<MemberBalanceChecked> {
        Ok(MemberBalanceChecked {
            addr: deps.api.addr_validate(&self.addr)?,
            balance: self.balance.into_checked(deps)?,
        })
    }
}
