use crate::state::{
    competition_categories, get_rulesets_category_and_is_enabled_idx, CompetitionModule, KEYS, TAX,
};
use arena_core_interface::msg::{
    CompetitionCategory, CompetitionModuleQuery, CompetitionModuleResponse, DumpStateResponse,
    Ruleset,
};
use cosmwasm_std::{Decimal, Deps, Empty, Env, StdResult, Uint128};
use cw_paginate::paginate_indexed_map;
use cw_storage_plus::Bound;
use cw_utils::maybe_addr;

impl CompetitionModule {
    pub fn to_response(&self, deps: Deps) -> StdResult<CompetitionModuleResponse<String>> {
        let competition_count: Uint128 = deps.querier.query_wasm_smart(
            self.addr.to_string(),
            &cw_competition::msg::QueryBase::<Empty, Empty, Empty>::CompetitionCount {},
        )?;

        Ok(CompetitionModuleResponse {
            key: self.key.clone(),
            addr: self.addr.to_string(),
            is_enabled: self.is_enabled,
            competition_count,
        })
    }
}

pub fn competition_modules(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    include_disabled: Option<bool>,
) -> StdResult<Vec<CompetitionModuleResponse<String>>> {
    let start_after_bound = maybe_addr(deps.api, start_after)?.map(Bound::exclusive);
    let limit = limit.unwrap_or(10).max(30);
    let include_disabled = include_disabled.unwrap_or(false);

    let competition_modules_map = crate::state::competition_modules();

    if include_disabled {
        cw_paginate::paginate_indexed_map(
            &competition_modules_map,
            deps.storage,
            start_after_bound,
            Some(limit),
            |_x, y| y.to_response(deps),
        )
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
            .map(|x| x.map(|y| y.1.to_response(deps)))
            .take(limit as usize)
            .try_fold(Vec::new(), |mut acc, res| {
                acc.push(res??);

                Ok(acc)
            })
    }
}

pub fn tax(deps: Deps, env: Env, height: Option<u64>) -> StdResult<Decimal> {
    Ok(TAX
        .may_load_at_height(deps.storage, height.unwrap_or(env.block.height))?
        .unwrap_or(Decimal::zero()))
}

pub fn rulesets(
    deps: Deps,
    category_id: Option<Uint128>,
    start_after: Option<Uint128>,
    limit: Option<u32>,
    include_disabled: Option<bool>,
) -> StdResult<Vec<Ruleset>> {
    let start_after_bound = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(10).max(30);
    let include_disabled = include_disabled.unwrap_or(false);

    let rulesets_map = crate::state::rulesets();

    let enabled_rulesets = rulesets_map
        .idx
        .category_and_is_enabled
        .prefix(get_rulesets_category_and_is_enabled_idx(category_id, true))
        .range(
            deps.storage,
            start_after_bound.clone(),
            None,
            cosmwasm_std::Order::Ascending,
        )
        .map(|x| x.map(|y| y.1));

    if include_disabled {
        let disabled_rulesets = rulesets_map
            .idx
            .category_and_is_enabled
            .prefix(get_rulesets_category_and_is_enabled_idx(category_id, false))
            .range(
                deps.storage,
                start_after_bound,
                None,
                cosmwasm_std::Order::Ascending,
            )
            .map(|x| x.map(|y| y.1));

        Ok(enabled_rulesets
            .chain(disabled_rulesets)
            .take(limit as usize)
            .collect::<StdResult<Vec<_>>>()?)
    } else {
        Ok(enabled_rulesets
            .take(limit as usize)
            .collect::<StdResult<Vec<_>>>()?)
    }
}

pub fn ruleset(deps: Deps, id: Uint128) -> StdResult<Option<Ruleset>> {
    crate::state::rulesets().may_load(deps.storage, id.u128())
}

pub fn categories(
    deps: Deps,
    start_after: Option<Uint128>,
    limit: Option<u32>,
    include_disabled: Option<bool>,
) -> StdResult<Vec<CompetitionCategory>> {
    let start_after_bound = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(10).max(30);
    let include_disabled = include_disabled.unwrap_or(false);

    let category_map = crate::state::competition_categories();

    if include_disabled {
        paginate_indexed_map(
            &category_map,
            deps.storage,
            start_after_bound,
            Some(limit),
            |_x, y| Ok(y),
        )
    } else {
        category_map
            .idx
            .is_enabled
            .prefix(true.to_string())
            .range(
                deps.storage,
                start_after_bound,
                None,
                cosmwasm_std::Order::Ascending,
            )
            .map(|x| x.map(|y| y.1))
            .take(limit as usize)
            .collect::<StdResult<Vec<_>>>()
    }
}

pub fn category(deps: Deps, id: Uint128) -> StdResult<Option<CompetitionCategory>> {
    crate::state::competition_categories().may_load(deps.storage, id.u128())
}

pub fn competition_module(
    deps: Deps,
    env: Env,
    query: CompetitionModuleQuery,
) -> StdResult<Option<CompetitionModuleResponse<String>>> {
    match query {
        CompetitionModuleQuery::Key(key, height) => {
            let height = height.unwrap_or(env.block.height);

            let maybe_addr = KEYS.may_load_at_height(deps.storage, key, height)?;

            match maybe_addr {
                Some(addr) => crate::state::competition_modules()
                    .may_load(deps.storage, addr)?
                    .map(|x| x.to_response(deps))
                    .transpose(),
                None => Ok(None),
            }
        }
        CompetitionModuleQuery::Addr(addr) => {
            let addr = deps.api.addr_validate(&addr)?;

            crate::state::competition_modules()
                .may_load(deps.storage, addr)?
                .map(|x| x.to_response(deps))
                .transpose()
        }
    }
}

pub fn dump_state(deps: Deps, env: Env) -> StdResult<DumpStateResponse> {
    Ok(DumpStateResponse {
        tax: tax(deps, env, None)?,
        competition_modules: competition_modules(deps, None, None, None)?,
    })
}

pub fn is_valid_category_and_rulesets(
    deps: Deps,
    category_id: Option<Uint128>,
    rulesets: Vec<Uint128>,
) -> bool {
    if let Some(category_id) = category_id {
        if !competition_categories().has(deps.storage, category_id.u128()) {
            return false;
        }
    }

    for ruleset_id in rulesets {
        if !crate::state::rulesets().has(deps.storage, ruleset_id.u128()) {
            return false;
        }

        match crate::state::rulesets().load(deps.storage, ruleset_id.u128()) {
            Ok(ruleset) => {
                if !ruleset.is_enabled {
                    return false;
                }
                if ruleset.category_id != category_id {
                    return false;
                }
            }
            Err(_) => return false,
        };
    }

    true
}
