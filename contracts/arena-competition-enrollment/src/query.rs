use std::collections::HashMap;

use arena_interface::core::{CompetitionModuleResponse, QueryExt};
use cosmwasm_std::{Addr, Deps, Order, StdError, StdResult, Uint128};
use cw_ownable::get_ownership;
use cw_storage_plus::Bound;

use crate::{
    msg::EnrollmentFilter,
    state::{enrollment_entries, EnrollmentEntryResponse},
};

pub fn module_map(deps: Deps) -> StdResult<HashMap<String, Addr>> {
    let ownership = get_ownership(deps.storage)?;

    if let Some(owner) = ownership.owner {
        let competition_modules = deps
            .querier
            .query_wasm_smart::<Vec<CompetitionModuleResponse<Addr>>>(
                &owner,
                &arena_interface::core::QueryMsg::QueryExtension {
                    msg: QueryExt::CompetitionModules {
                        start_after: None,
                        limit: None,
                        include_disabled: None,
                    },
                },
            )?;

        Ok(competition_modules
            .into_iter()
            .map(|x| (x.key, x.addr))
            .collect())
    } else {
        Err(StdError::generic_err("No owner"))
    }
}

pub fn enrollments(
    deps: Deps,
    start_after: Option<Uint128>,
    limit: Option<u32>,
    filter: Option<EnrollmentFilter>,
) -> StdResult<Vec<EnrollmentEntryResponse>> {
    let start_after_bound = start_after.map(|x| x.u128()).map(Bound::exclusive);
    let limit = limit.unwrap_or(10).max(30);

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
