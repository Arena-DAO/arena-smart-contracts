use cosmwasm_std::{Deps, Order, StdResult};
use cw4::{MemberListResponse, MemberResponse, TotalWeightResponse};
use cw_storage_plus::Bound;
use cw_utils::maybe_addr;

use crate::state::{members, TOTAL_POWER};

// Constants for pagination
const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

pub fn total_weight(deps: Deps, height: Option<u64>) -> StdResult<cosmwasm_std::Binary> {
    let weight = match height {
        Some(h) => TOTAL_POWER.may_load_at_height(deps.storage, h),
        None => TOTAL_POWER.may_load(deps.storage),
    }?
    .unwrap_or_default()
    .u64();
    cosmwasm_std::to_json_binary(&TotalWeightResponse { weight })
}

pub fn list_members(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<cosmwasm_std::Binary> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = maybe_addr(deps.api, start_after)?;
    let start = addr.as_ref().map(Bound::exclusive);

    let members_map = members();
    let members_list: Vec<_> = members_map
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|(addr, data)| cw4::Member {
                addr: addr.to_string(),
                weight: data.power.into(),
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    cosmwasm_std::to_json_binary(&MemberListResponse {
        members: members_list,
    })
}

pub fn member(deps: Deps, addr: String, height: Option<u64>) -> StdResult<MemberResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    let weight = match height {
        Some(h) => members().may_load_at_height(deps.storage, &addr, h),
        None => members().may_load(deps.storage, &addr),
    }?
    .map(|x| x.power.u64());
    Ok(MemberResponse { weight })
}
