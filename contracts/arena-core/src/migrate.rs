use arena_interface::fees::TaxConfiguration;
use cosmwasm_std::{from_json, DepsMut, StdError, Uint128};
use cw_utils::Duration;

use crate::{
    state::{ARENA_TAX_CONFIG, COMPETITION_CATEGORIES_COUNT, RATING_PERIOD},
    ContractError,
};

pub fn from_v1_3_to_v1_4(deps: DepsMut) -> Result<(), ContractError> {
    let prev_key = "competition-categories-count".as_bytes();

    let competition_categories_count: Uint128 = from_json(
        deps.storage
            .get(prev_key)
            .ok_or_else(|| StdError::not_found("State"))?,
    )?;
    deps.storage.remove(prev_key);
    COMPETITION_CATEGORIES_COUNT.save(deps.storage, &competition_categories_count)?;

    ARENA_TAX_CONFIG.save(
        deps.storage,
        &TaxConfiguration {
            cw20_msg: None,
            cw721_msg: None,
        },
    )?;

    Ok(())
}

pub fn from_v1_4_to_v1_6(deps: DepsMut) -> Result<(), ContractError> {
    // Default the rating period to 1 week
    RATING_PERIOD.save(deps.storage, &Duration::Time(604800u64))?;

    Ok(())
}
