use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::{
    state::{VestingConfiguration, VESTING_CONFIGURATION},
    ContractError,
};

pub fn update_vesting_configuration(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    config: VestingConfiguration,
) -> Result<Response, ContractError> {
    if info.sender != env.contract.address {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;
    }

    config.into_checked()?;

    VESTING_CONFIGURATION.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_vesting_configuration")
        .add_attributes(config.into_attributes()))
}
