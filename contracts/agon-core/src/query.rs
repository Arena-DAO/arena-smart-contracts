use crate::{
    models::{CompetitionModuleResponse, DumpStateResponse},
    state::COMPETITION_MODULES,
};
use cosmwasm_std::{Deps, StdResult};
use cw_paginate::paginate_map;

pub fn dump_state(deps: Deps) -> StdResult<DumpStateResponse> {
    Ok(DumpStateResponse {
        competition_modules: competition_modules(deps, None, None)?,
    })
}

pub fn competition_modules(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<CompetitionModuleResponse>> {
    let start_after = start_after
        .map(|x| deps.api.addr_validate(&x))
        .transpose()?;

    Ok(paginate_map(
        deps,
        &COMPETITION_MODULES,
        start_after,
        limit,
        cosmwasm_std::Order::Descending,
    )?
    .iter()
    .map(|x| CompetitionModuleResponse {
        addr: x.0.clone(),
        info: x.1.clone(),
    })
    .collect())
}
