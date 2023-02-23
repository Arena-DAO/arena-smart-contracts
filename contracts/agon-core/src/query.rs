use std::convert::TryInto;

use crate::{
    models::{CompetitionModule, DumpStateResponse, Ruleset, Wager},
    state::{rulesets, COMPETITION_MODULES, TAX, WAGERS},
};
use cosmwasm_std::{Decimal, Deps, Env, Order, StdResult};
use cw_paginate::paginate_map_values;
use cw_storage_plus::PrefixBound;

pub fn dump_state(deps: Deps) -> StdResult<DumpStateResponse> {
    Ok(DumpStateResponse {
        competition_modules: competition_modules(deps, None, None)?,
    })
}

pub fn competition_modules(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<CompetitionModule>> {
    let start_after = start_after
        .map(|x| deps.api.addr_validate(&x))
        .transpose()?;

    Ok(paginate_map_values(
        deps,
        &COMPETITION_MODULES,
        start_after,
        limit,
        cosmwasm_std::Order::Descending,
    )?)
}

pub fn tax(deps: Deps, env: Env, height: Option<u64>) -> StdResult<Decimal> {
    Ok(TAX
        .may_load_at_height(deps.storage, height.unwrap_or(env.block.height))?
        .unwrap_or(Decimal::zero()))
}

pub fn rulesets_by_description(
    deps: Deps,
    skip: Option<u32>,
    limit: Option<u32>,
    description: Option<String>,
) -> StdResult<Vec<Ruleset>> {
    let limit = limit.unwrap_or(16u32);
    let skip = skip.unwrap_or_default();

    Ok(rulesets()
        .idx
        .description
        .prefix_range(
            deps.storage,
            description.map(PrefixBound::exclusive),
            None,
            Order::Ascending,
        )
        .skip(skip.try_into().unwrap())
        .take(limit.try_into().unwrap())
        .map(|x| x.map(|y| y.1))
        .collect::<StdResult<_>>()?)
}

pub fn wager(deps: Deps, id: u128) -> StdResult<Wager> {
    Ok(WAGERS.load(deps.storage, id)?)
}
