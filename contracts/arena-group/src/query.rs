use cosmwasm_std::{Addr, Deps, Order, StdResult, Uint64};
use cw_storage_plus::Bound;

use crate::state::{members as members_map, MEMBER_COUNT};

pub fn members(
    deps: Deps,
    start_after: Option<(Uint64, String)>,
    limit: Option<u32>,
) -> StdResult<Vec<Addr>> {
    let binding = start_after
        .as_ref()
        .map(|(_seed, addr)| deps.api.addr_validate(addr))
        .transpose()?;
    let start_after = start_after
        .map(|(seed, _addr)| (seed.u64(), binding.as_ref().unwrap()))
        .map(Bound::exclusive);
    let limit = limit.map(|x| x as usize).unwrap_or(usize::MAX);

    members_map()
        .idx
        .seed
        .range(deps.storage, start_after, None, Order::Ascending)
        .map(|x| x.map(|(addr, _seed)| addr))
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
        .map(|x| deps.api.addr_validate(&x))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(addrs.iter().all(|x| members_map().has(deps.storage, x)))
}
