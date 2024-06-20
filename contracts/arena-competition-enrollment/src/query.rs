use cosmwasm_std::{Deps, Env, StdResult, Uint128};
use cw_storage_plus::Bound;

use crate::{
    msg::EnrollmentFilter,
    state::{enrollment_entries, EnrollmentEntryResponse},
};

pub fn enrollments(
    deps: Deps,
    env: Env,
    start_after: Option<(Uint128, Uint128)>,
    limit: Option<u32>,
    filter: Option<EnrollmentFilter>,
) -> StdResult<Vec<EnrollmentEntryResponse>> {
    let start_after_bound = start_after
        .map(|(category_id, enrollment_id)| (category_id.u128(), enrollment_id.u128()))
        .map(Bound::exclusive);
    let limit = limit.unwrap_or(10).max(30);

    match filter {
        None => cw_paginate::paginate_indexed_map(
            &enrollment_entries(),
            deps.storage,
            start_after_bound,
            Some(limit),
            |_x, y| Ok(y.into_list_item_response(&env.block)),
        ),
        Some(filter) => match filter {
            EnrollmentFilter::Expiration {} => enrollment_entries()
                .idx
                .expiration
                .range(
                    deps.storage,
                    start_after_bound,
                    None,
                    cosmwasm_std::Order::Descending,
                )
                .map(|x| x.map(|y| y.1.into_list_item_response(&env.block)))
                .take(limit as usize)
                .collect::<StdResult<Vec<_>>>(),
            EnrollmentFilter::Host(addr) => enrollment_entries()
                .idx
                .host
                .prefix(addr)
                .range(
                    deps.storage,
                    start_after_bound,
                    None,
                    cosmwasm_std::Order::Descending,
                )
                .map(|x| x.map(|y| y.1.into_list_item_response(&env.block)))
                .take(limit as usize)
                .collect::<StdResult<Vec<_>>>(),
        },
    }
}
