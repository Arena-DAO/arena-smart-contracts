use cosmwasm_std::{Addr, Deps, Env, StdResult};
use cw_balance::Distribution;

use crate::state::PRESET_DISTRIBUTIONS;

pub fn get_distribution(
    deps: Deps,
    env: Env,
    addr: String,
    height: Option<u64>,
) -> StdResult<Option<Distribution<Addr>>> {
    let addr = deps.api.addr_validate(&addr)?;
    let height = height.unwrap_or(env.block.height);

    PRESET_DISTRIBUTIONS.may_load_at_height(deps.storage, &addr, height)
}
