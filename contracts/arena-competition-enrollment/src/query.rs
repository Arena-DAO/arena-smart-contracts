use cosmwasm_std::{Addr, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

use crate::{
    msg::EnrollmentFilter,
    state::{enrollment_entries, EnrollmentEntryResponse, ENROLLMENT_COUNT, ENROLLMENT_MEMBERS},
};

pub fn enrollments(
    deps: Deps,
    env: Env,
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
            |x, y| y.into_response(deps, &env.block, Uint128::new(x)),
        ),
        Some(filter) => match filter {
            EnrollmentFilter::Category { category_id } => enrollment_entries()
                .idx
                .category
                .prefix(category_id.unwrap_or(Uint128::zero()).u128())
                .range(deps.storage, start_after_bound, None, Order::Descending)
                .map(|x| x.map(|y| y.1.into_response(deps, &env.block, Uint128::new(y.0)))?)
                .take(limit as usize)
                .collect::<StdResult<Vec<_>>>(),
            EnrollmentFilter::Host(addr) => enrollment_entries()
                .idx
                .host
                .prefix(addr)
                .range(deps.storage, start_after_bound, None, Order::Descending)
                .map(|x| x.map(|y| y.1.into_response(deps, &env.block, Uint128::new(y.0)))?)
                .take(limit as usize)
                .collect::<StdResult<Vec<_>>>(),
        },
    }
}

pub fn enrollment_count(deps: Deps) -> StdResult<Uint128> {
    Ok(ENROLLMENT_COUNT.may_load(deps.storage)?.unwrap_or_default())
}

pub fn enrollment_members(
    deps: Deps,
    enrollment_id: Uint128,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<Addr>> {
    let binding = start_after
        .map(|x| deps.api.addr_validate(&x))
        .transpose()?;
    let start_after_bound = binding.as_ref().map(Bound::exclusive);
    let limit = limit.unwrap_or(30).max(30);

    ENROLLMENT_MEMBERS
        .prefix(enrollment_id.u128())
        .range(deps.storage, start_after_bound, None, Order::Descending)
        .map(|x| x.map(|y| y.0))
        .take(limit as usize)
        .collect::<StdResult<Vec<_>>>()
}

pub fn is_member(deps: Deps, enrollment_id: Uint128, addr: String) -> StdResult<bool> {
    let addr = deps.api.addr_validate(&addr)?;

    Ok(ENROLLMENT_MEMBERS.has(deps.storage, (enrollment_id.u128(), &addr)))
}
