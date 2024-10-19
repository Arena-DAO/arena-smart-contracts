use cosmwasm_std::DepsMut;

use crate::ContractError;

pub fn from_v1_8_2_to_v2(deps: DepsMut) -> Result<(), ContractError> {
    deps.storage.remove(b"should_activate_on_funded");
    deps.storage.remove(b"distribution");

    Ok(())
}

pub fn from_v2_to_v2_1(deps: DepsMut) -> Result<(), ContractError> {
    deps.storage.remove(b"deferred_fees");

    Ok(())
}
