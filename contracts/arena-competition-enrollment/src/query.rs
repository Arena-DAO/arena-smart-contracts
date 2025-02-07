use arena_interface::{competition::types::EnrollmentEntryResponse, group};
use cosmwasm_std::{Deps, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

use crate::{
    msg::EnrollmentFilter,
    state::{enrollment_entries, ENROLLMENT_COUNT},
};

pub fn enrollments(
    deps: Deps,
    start_after: Option<Uint128>,
    limit: Option<u32>,
    filter: Option<EnrollmentFilter>,
) -> StdResult<Vec<EnrollmentEntryResponse>> {
    let start_after_bound = start_after.map(|x| x.u128()).map(Bound::exclusive);
    let limit = limit.unwrap_or(30).max(30);

    match filter {
        None => cw_paginate::paginate_indexed_map(
            &enrollment_entries(),
            deps.storage,
            start_after_bound,
            Some(limit),
            |x, y| y.into_response(deps, Uint128::new(x)),
        ),
        Some(filter) => match filter {
            EnrollmentFilter::Category { category_id } => enrollment_entries()
                .idx
                .category
                .prefix(category_id.unwrap_or(Uint128::zero()).u128())
                .range(deps.storage, start_after_bound, None, Order::Descending)
                .map(|x| x.map(|y| y.1.into_response(deps, Uint128::new(y.0)))?)
                .take(limit as usize)
                .collect::<StdResult<Vec<_>>>(),
            EnrollmentFilter::Host(addr) => enrollment_entries()
                .idx
                .host
                .prefix(addr)
                .range(deps.storage, start_after_bound, None, Order::Descending)
                .map(|x| x.map(|y| y.1.into_response(deps, Uint128::new(y.0)))?)
                .take(limit as usize)
                .collect::<StdResult<Vec<_>>>(),
        },
    }
}

pub fn enrollment_count(deps: Deps) -> StdResult<Uint128> {
    Ok(ENROLLMENT_COUNT.may_load(deps.storage)?.unwrap_or_default())
}

pub fn is_member(deps: Deps, enrollment_id: Uint128, addr: String) -> StdResult<bool> {
    let enrollment = enrollment_entries().load(deps.storage, enrollment_id.u128())?;
    let group_contract = enrollment
        .competition_info
        .into_response(deps, &enrollment.competition_module)?
        .group_contract;

    deps.querier
        .query_wasm_smart::<bool>(group_contract, &group::QueryMsg::IsMember { addr })
}
