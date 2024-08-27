use std::fmt;

use cosmwasm_std::{Addr, Attribute, Decimal, Deps, StdError, StdResult};
use dao_interface::query::GetItemResponse;
use serde_json::Value;

use crate::{
    state::{ApplicationStatus, VestingConfiguration},
    ContractError,
};

impl VestingConfiguration {
    pub fn into_checked(&self) -> StdResult<()> {
        // Validate upfront_ratio
        if self.upfront_ratio > Decimal::one() || self.upfront_ratio == Decimal::zero() {
            return Err(StdError::generic_err(
                "Upfront ratio must be between 0 and 1",
            ));
        }

        // Validate vesting_time
        if self.vesting_time == 0 {
            return Err(StdError::generic_err("Vesting time must be greater than 0"));
        }

        // All checks passed
        Ok(())
    }

    pub fn into_attributes(&self) -> Vec<Attribute> {
        vec![
            Attribute::new("upfront_ratio", self.upfront_ratio.to_string()),
            Attribute::new("vesting_time", self.vesting_time.to_string()),
            Attribute::new("denom", self.denom.clone()),
        ]
    }
}

impl fmt::Display for ApplicationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplicationStatus::Pending {} => write!(f, "pending"),
            ApplicationStatus::Accepted {} => write!(f, "accepted"),
            ApplicationStatus::Rejected { .. } => write!(f, "rejected"),
        }
    }
}

pub fn get_payroll_address(deps: Deps, chain_id: &str) -> Result<Addr, ContractError> {
    let ownership = cw_ownable::get_ownership(deps.storage)?;

    let owner = ownership.owner.ok_or(ContractError::OwnershipError(
        cw_ownable::OwnershipError::NoOwner,
    ))?;

    let get_item_response: GetItemResponse = deps.querier.query_wasm_smart(
        owner.to_string(),
        &dao_interface::msg::QueryMsg::GetItem {
            key: "widget:vesting".to_owned(),
        },
    )?;

    let value = get_item_response.item.ok_or_else(|| {
        ContractError::StdError(StdError::generic_err("Could not find the payroll factory"))
    })?;

    let v: Value = serde_json::from_str(&value)
        .map_err(|e| ContractError::StdError(StdError::parse_err("JSON", e.to_string())))?;

    let address = v["factories"][chain_id]["address"]
        .as_str()
        .ok_or_else(|| {
            ContractError::StdError(StdError::generic_err("Address not found or not a string"))
        })?;

    deps.api
        .addr_validate(address)
        .map_err(ContractError::StdError)
}
