use cosmwasm_std::{Deps, StdResult};
use cw_balance::{BalanceVerified, MemberShareVerified};

use crate::{
    msg::DumpStateResponse,
    state::{ADMIN, BALANCE, DUE, IS_FUNDED, IS_LOCKED, PRESET_DISTRIBUTION, TOTAL_BALANCE},
};

pub fn balance(deps: Deps, addr: String) -> StdResult<BalanceVerified> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(BALANCE.may_load(deps.storage, &addr)?.unwrap_or_default())
}

pub fn due(deps: Deps, addr: String) -> StdResult<BalanceVerified> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(DUE.may_load(deps.storage, &addr)?.unwrap_or_default())
}

pub fn total_balance(deps: Deps) -> BalanceVerified {
    TOTAL_BALANCE.load(deps.storage).unwrap_or_default()
}

pub fn is_locked(deps: Deps) -> bool {
    IS_LOCKED.load(deps.storage).unwrap_or_default()
}

pub fn dump_state(deps: Deps) -> StdResult<DumpStateResponse> {
    Ok(DumpStateResponse {
        admin: ADMIN.get(deps)?.unwrap(),
        is_locked: is_locked(deps),
        total_balance: total_balance(deps),
    })
}

pub fn distribution(deps: Deps, addr: String) -> StdResult<Option<Vec<MemberShareVerified>>> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(PRESET_DISTRIBUTION.may_load(deps.storage, &addr)?)
}

pub fn is_funded(deps: Deps, addr: String) -> StdResult<bool> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(IS_FUNDED.load(deps.storage, &addr).unwrap_or_default())
}

pub fn is_fully_funded(deps: Deps) -> StdResult<bool> {
    Ok(crate::state::is_fully_funded(deps)?)
}
