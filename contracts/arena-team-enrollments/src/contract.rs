use crate::execute::{
    apply, create_entry, update_applicant_status, update_entry_status, withdraw_application,
};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::query::{get_applicant, get_entry, list_applicants, list_entries};
use crate::state::TEAM_ENTRY_COUNT;
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult,
};
use cw2::set_contract_version;

// version info for migration and contract info
const CONTRACT_NAME: &str = "crates.io:arena-team-enrollments";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Instantiate contract
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    TEAM_ENTRY_COUNT.save(deps.storage, &0)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;
    Ok(Response::new().add_attribute("action", "instantiate"))
}

/// Execute entry point
#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::CreateEntry {
            title,
            description,
            category_id,
            dao_config,
        } => create_entry(deps, env, info, title, description, category_id, dao_config),
        ExecuteMsg::UpdateEntryStatus { entry_id, status } => {
            update_entry_status(deps, info, entry_id, status)
        }
        ExecuteMsg::Apply { entry_id } => apply(deps, env, info, entry_id),
        ExecuteMsg::WithdrawApplication { entry_id } => withdraw_application(deps, info, entry_id),
        ExecuteMsg::UpdateApplicantStatus {
            entry_id,
            applicant,
            status,
        } => update_applicant_status(deps, info, entry_id, applicant, status),
        ExecuteMsg::UpdateOwnership(action) => {
            let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)
                .map_err(|e| StdError::generic_err(e.to_string()))?;
            Ok(Response::new().add_attributes(ownership.into_attributes()))
        }
    }
}

/// Query entry point
#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetEntry { entry_id } => to_json_binary(&get_entry(deps, entry_id)?),
        QueryMsg::ListEntries {
            category_id,
            status,
            start_after,
            limit,
        } => to_json_binary(&list_entries(
            deps,
            category_id,
            status,
            start_after,
            limit,
        )?),
        QueryMsg::GetApplicant {
            entry_id,
            applicant,
        } => to_json_binary(&get_applicant(deps, entry_id, applicant)?),
        QueryMsg::ListApplicants {
            entry_id,
            start_after,
            limit,
        } => to_json_binary(&list_applicants(deps, entry_id, start_after, limit)?),
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
    }
}
