use arena_interface::group::MemberMsg;
use cosmwasm_std::{Addr, Deps, Order, StdResult};
use cw_storage_plus::Bound;

use crate::state::{members as members_map, MEMBER_COUNT};

pub fn members(
    deps: Deps,
    start_after: Option<MemberMsg<String>>,
    limit: Option<u32>,
) -> StdResult<Vec<MemberMsg<Addr>>> {
    let binding = start_after
        .as_ref()
        .map(|MemberMsg { addr, seed: _ }| deps.api.addr_validate(addr))
        .transpose()?;
    let start_after = start_after
        .map(|MemberMsg { addr: _, seed }| (seed.u64(), binding.as_ref().unwrap()))
        .map(Bound::exclusive);
    let limit = limit.map(|x| x as usize).unwrap_or(usize::MAX);

    members_map()
        .idx
        .seed
        .range(deps.storage, start_after, None, Order::Ascending)
        .map(|x| x.map(|(addr, seed)| MemberMsg { addr, seed }))
        .take(limit)
        .collect()
}

pub fn is_valid_distribution(deps: Deps, addrs: Vec<String>) -> StdResult<bool> {
    if addrs.is_empty() {
        return Ok(false);
    }

    if MEMBER_COUNT
        .may_load(deps.storage)?
        .unwrap_or_default()
        .is_zero()
    {
        return Ok(true);
    }

    let addrs = addrs
        .iter()
        .map(|x| deps.api.addr_validate(x))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(addrs.iter().all(|x| members_map().has(deps.storage, x)))
}

pub fn is_member(deps: Deps, addr: String) -> StdResult<bool> {
    Ok(members_map().has(deps.storage, &deps.api.addr_validate(&addr)?))
}
