use crate::state::{CompetitionModule, Ruleset, KEYS, TAX};
use cosmwasm_std::{Addr, Decimal, Deps, Env, StdResult};
use cw_storage_plus::Bound;
use cw_utils::maybe_addr;

pub fn competition_modules(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    include_disabled: Option<bool>,
) -> StdResult<Vec<(Addr, CompetitionModule)>> {
    let start_after_bound = maybe_addr(deps.api, start_after)?.map(Bound::exclusive);
    let limit = limit.unwrap_or(10).max(30) as usize;
    let include_disabled = include_disabled.unwrap_or(false);

    let competition_modules_map = crate::state::competition_modules();

    let items = if include_disabled {
        competition_modules_map
            .range(
                deps.storage,
                start_after_bound,
                None,
                cosmwasm_std::Order::Ascending,
            )
            .take(limit)
            .collect::<StdResult<Vec<_>>>()?
    } else {
        competition_modules_map
            .idx
            .is_enabled
            .prefix(true.to_string())
            .range(
                deps.storage,
                start_after_bound,
                None,
                cosmwasm_std::Order::Ascending,
            )
            .take(limit)
            .collect::<StdResult<Vec<_>>>()?
    };

    Ok(items)
}

pub fn tax(deps: Deps, env: Env, height: Option<u64>) -> StdResult<Decimal> {
    Ok(TAX
        .may_load_at_height(deps.storage, height.unwrap_or(env.block.height))?
        .unwrap_or(Decimal::zero()))
}

pub fn rulesets(
    deps: Deps,
    start_after: Option<u128>,
    limit: Option<u32>,
    include_disabled: Option<bool>,
) -> StdResult<Vec<(u128, Ruleset)>> {
    let start_after_bound = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(10).max(30);
    let include_disabled = include_disabled.unwrap_or(false);

    let rulesets_map = crate::state::rulesets();

    if include_disabled {
        cw_paginate::paginate_indexed_map(
            &rulesets_map,
            deps.storage,
            start_after_bound,
            Some(limit),
            |x, y| Ok((x, y)),
        )
    } else {
        rulesets_map
            .idx
            .is_enabled
            .prefix(true.to_string())
            .range(
                deps.storage,
                start_after_bound,
                None,
                cosmwasm_std::Order::Ascending,
            )
            .take(limit as usize)
            .collect::<StdResult<Vec<_>>>()
    }
}

pub fn competition_module(deps: Deps, key: String) -> StdResult<Option<Addr>> {
    KEYS.may_load(deps.storage, key)
}
