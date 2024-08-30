use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdResult};
use cw_balance::Distribution;

use crate::state::PRESET_DISTRIBUTIONS;

pub fn set_distribution(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    distribution: Distribution<String>,
) -> StdResult<Response> {
    let distribution = distribution.into_checked(deps.as_ref())?;

    PRESET_DISTRIBUTIONS.save(deps.storage, &info.sender, &distribution, env.block.height)?;

    Ok(Response::new().add_attribute("action", "set_distribution"))
}

pub fn remove_distribution(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<Response> {
    PRESET_DISTRIBUTIONS.remove(deps.storage, &info.sender, env.block.height)?;

    Ok(Response::new().add_attribute("action", "remove_distribution"))
}
