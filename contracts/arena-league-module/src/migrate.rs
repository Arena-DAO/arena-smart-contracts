use arena_interface::competition::state::Config;
use cosmwasm_std::{DepsMut, Empty};

use crate::{contract::CompetitionModule, ContractError};

pub fn from_v1_3_to_v_1_4(deps: DepsMut) -> Result<(), ContractError> {
    CompetitionModule::default().config.save(
        deps.storage,
        &Config {
            key: "Arena League Module".to_string(),
            description: "A competition module for handling round-robin tournaments".to_string(),
            extension: Empty {},
        },
    )?;

    Ok(())
}
