use arena_interface::competition::types::DaoConfig;
use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128};

use crate::state::{
    applicants, team_entries, ApplicantStatus, EntryStatus, TeamEntry, TEAM_ENTRY_COUNT,
};

pub fn create_entry(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    title: String,
    description: String,
    category_id: Option<Uint128>,
    dao_config: DaoConfig,
) -> StdResult<Response> {
    let mut id = TEAM_ENTRY_COUNT.load(deps.storage).unwrap_or(0);
    id += 1;
    TEAM_ENTRY_COUNT.save(deps.storage, &id)?;

    let entry = TeamEntry {
        creator: info.sender.clone(),
        title,
        description,
        category_id,
        status: EntryStatus::Open,
        created_at: env.block.time,
        dao_config,
    };

    team_entries().save(deps.storage, id, &entry)?;

    Ok(Response::new()
        .add_attribute("action", "create_entry")
        .add_attribute("entry_id", id.to_string())
        .add_attribute("creator", info.sender))
}

pub fn update_entry_status(
    deps: DepsMut,
    info: MessageInfo,
    entry_id: u64,
    new_status: EntryStatus,
) -> StdResult<Response> {
    let entry = team_entries().update(deps.storage, entry_id, |maybe_entry| {
        let mut entry = maybe_entry.ok_or(StdError::not_found("TeamEntry"))?;
        if entry.creator != info.sender {
            return Err(StdError::generic_err("Unauthorized"));
        }
        entry.status.validate_transition(&new_status)?;

        entry.status = new_status;
        Ok(entry)
    })?;

    if matches!(entry.status, EntryStatus::Created) {
        // Create the team
    }

    Ok(Response::new()
        .add_attribute("action", "update_entry_status")
        .add_attribute("entry_id", entry_id.to_string()))
}

pub fn apply_to_entry(deps: DepsMut, info: MessageInfo, entry_id: u64) -> StdResult<Response> {
    let entry = team_entries().load(deps.storage, entry_id)?;
    if entry.status != EntryStatus::Open {
        return Err(StdError::generic_err("Entry is not open for applications"));
    }

    let applicant_key = (entry_id, info.sender.clone());

    if applicants()
        .may_load(deps.storage, applicant_key.clone())?
        .is_some()
    {
        return Err(StdError::generic_err("Already applied"));
    }

    applicants().save(deps.storage, applicant_key, &ApplicantStatus::Default)?;

    Ok(Response::new()
        .add_attribute("action", "apply")
        .add_attribute("entry_id", entry_id.to_string())
        .add_attribute("applicant", info.sender))
}

pub fn withdraw_application(
    deps: DepsMut,
    info: MessageInfo,
    entry_id: u64,
) -> StdResult<Response> {
    let key = (entry_id, info.sender.clone());

    let status = applicants()
        .may_load(deps.storage, key.clone())?
        .ok_or_else(|| StdError::not_found("Application"))?;

    if matches!(status, ApplicantStatus::Rejected { .. }) {
        return Err(StdError::generic_err(
            "Rejected applications cannot be withdrawn",
        ));
    }

    Ok(Response::new()
        .add_attribute("action", "withdraw_application")
        .add_attribute("entry_id", entry_id.to_string())
        .add_attribute("applicant", info.sender))
}

pub fn update_applicant_status(
    deps: DepsMut,
    info: MessageInfo,
    entry_id: u64,
    applicant: Addr,
    new_status: ApplicantStatus,
) -> StdResult<Response> {
    let entry = team_entries().load(deps.storage, entry_id)?;
    if entry.creator != info.sender {
        return Err(StdError::generic_err("Unauthorized"));
    }

    let key = (entry_id, applicant.clone());

    let current_status = applicants()
        .may_load(deps.storage, key.clone())?
        .ok_or_else(|| StdError::not_found("Application"))?;

    current_status.validate_transition(&new_status)?;

    applicants().save(deps.storage, key, &new_status)?;

    Ok(Response::new()
        .add_attribute("action", "update_applicant_status")
        .add_attribute("entry_id", entry_id.to_string())
        .add_attribute("applicant", applicant.to_string())
        .add_attribute("new_status", new_status.to_string()))
}
