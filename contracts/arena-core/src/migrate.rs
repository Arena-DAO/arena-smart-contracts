use arena_core_interface::fees::TaxConfiguration;
use cosmwasm_std::{from_json, DepsMut, StdError, Uint128};

use crate::{
    state::{ARENA_TAX_CONFIG, COMPETITION_CATEGORIES_COUNT, COMPETITION_MODULES_COUNT},
    ContractError,
};

pub fn from_v1_3_to_v_1_4(deps: DepsMut) -> Result<(), ContractError> {
    let prev_key = "competition-categories-count".as_bytes();

    let competition_categories_count: Uint128 = from_json(
        deps.storage
            .get(prev_key)
            .ok_or_else(|| StdError::not_found("State"))?,
    )?;
    deps.storage.remove(prev_key);
    COMPETITION_CATEGORIES_COUNT.save(deps.storage, &competition_categories_count)?;

    let prev_key = "competition-modules-count".as_bytes();

    let competition_modules_count: Uint128 = from_json(
        deps.storage
            .get(prev_key)
            .ok_or_else(|| StdError::not_found("State"))?,
    )?;
    deps.storage.remove(prev_key);
    COMPETITION_MODULES_COUNT.save(deps.storage, &competition_modules_count)?;

    ARENA_TAX_CONFIG.save(
        deps.storage,
        &TaxConfiguration {
            cw20_msg: None,
            cw721_msg: None,
        },
    )?;

    Ok(())
}
