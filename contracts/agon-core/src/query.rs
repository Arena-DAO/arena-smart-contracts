use crate::{
    models::{CompetitionModule, DumpStateResponse},
    state::COMPETITION_MODULES,
};
use cosmwasm_std::{Deps, StdResult};
use cw_paginate::paginate_map_values;

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
