use cosmwasm_std::{to_json_binary, DepsMut, Env, MessageInfo, Response, WasmMsg};
use cw_ownable::assert_owner;
use cw_vesting::vesting::Schedule;

use crate::{
    contract::CONTRACT_NAME,
    msg::ApplyMsg,
    state::{
        applications, ApplicationInfo, ApplicationStatus, VestingConfiguration,
        VESTING_CONFIGURATION,
    },
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

pub fn apply(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ApplyMsg,
) -> Result<Response, ContractError> {
    let applicant = info.sender;

    // Check if the application already exists
    if applications().has(deps.storage, &applicant) {
        return Err(ContractError::ApplicationAlreadyExists {});
    }

    // Create the application info
    let application_info = ApplicationInfo {
        title: msg.title,
        description: msg.description,
        requested_amount: msg.requested_amount,
        project_links: msg.project_links,
        status: ApplicationStatus::Pending {},
    };

    // Save the application
    applications().save(deps.storage, &applicant, &application_info)?;

    Ok(Response::new()
        .add_attribute("action", "apply")
        .add_attribute("applicant", applicant))
}

pub fn accept_application(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    applicant: String,
) -> Result<Response, ContractError> {
    // Assert that the sender is the contract owner
    assert_owner(deps.storage, &info.sender)?;

    // Validate and convert the applicant address
    let applicant_addr = deps.api.addr_validate(&applicant)?;

    // Load the application
    let mut application = applications().load(deps.storage, &applicant_addr)?;

    // Check if the application is in the Pending state
    if !matches!(application.status, ApplicationStatus::Pending {}) {
        return Err(ContractError::InvalidApplicationStatus {});
    }

    // Update the application status to Accepted
    application.status = ApplicationStatus::Accepted {};
    applications().save(deps.storage, &applicant_addr, &application)?;

    // Load the vesting configuration
    let vesting_config = VESTING_CONFIGURATION.load(deps.storage)?;

    // Calculate the vesting amount (total requested amount minus the upfront amount)
    let upfront_amount = application.requested_amount * vesting_config.upfront_ratio;
    let vesting_amount = application.requested_amount - upfront_amount;

    // Prepare the instantiate message for the vesting contract
    let vesting_msg = WasmMsg::Instantiate {
        admin: Some(info.sender.to_string()),
        code_id: vesting_config.cw_vesting_code_id,
        msg: to_json_binary(&cw_vesting::msg::InstantiateMsg {
            owner: Some(info.sender.to_string()),
            recipient: applicant.clone(),
            title: application.title.clone(),
            description: Some(application.description.clone()),
            total: vesting_amount,
            denom: cw_vesting::UncheckedDenom::Native(vesting_config.denom.clone()),
            schedule: Schedule::SaturatingLinear,
            start_time: Some(env.block.time),
            vesting_duration_seconds: 31_536_000, // 365 days * 24 hours * 60 minutes * 60 seconds
            unbonding_duration_seconds: 31_536_000,
        })?,
        funds: vec![],
        label: CONTRACT_NAME.to_string(),
    };

    Ok(Response::new()
        .add_message(vesting_msg)
        .add_attribute("action", "accept_application")
        .add_attribute("applicant", applicant)
        .add_attribute("upfront_amount", upfront_amount.to_string())
        .add_attribute("vesting_amount", vesting_amount.to_string()))
}

pub fn reject_application(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    applicant: String,
    reason: Option<String>,
) -> Result<Response, ContractError> {
    // Assert that the sender is the contract owner
    assert_owner(deps.storage, &info.sender)?;

    // Validate and convert the applicant address
    let applicant_addr = deps.api.addr_validate(&applicant)?;

    // Load the application
    let mut application = applications().load(deps.storage, &applicant_addr)?;

    // Check if the application is in the Pending state
    if !matches!(application.status, ApplicationStatus::Pending {}) {
        return Err(ContractError::InvalidApplicationStatus {});
    }

    // Update the application status to Rejected
    application.status = ApplicationStatus::Rejected {
        reason: reason.clone(),
    };
    applications().save(deps.storage, &applicant_addr, &application)?;

    Ok(Response::new()
        .add_attribute("action", "reject_application")
        .add_attribute("applicant", applicant)
        .add_attribute(
            "reason",
            reason.unwrap_or_else(|| "No reason provided".to_string()),
        ))
}
