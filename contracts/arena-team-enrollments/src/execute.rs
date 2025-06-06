use cosmwasm_std::{
    instantiate2_address, to_json_binary, DepsMut, Empty, Env, MessageInfo, Order, Response,
    StdError, StdResult, Uint128, WasmMsg,
};
use cw4::Member;
use dao_interface::state::{Admin, ModuleInstantiateInfo};
use sha2::{Digest, Sha256};

use crate::state::{
    applicants, team_entries, ApplicantStatus, EntryStatus, TeamDaoConfig, TeamEntry,
    APPLICANTS_COUNT, TEAM_ENTRY_COUNT, USER_TEAMS,
};

pub fn create_entry(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    title: String,
    description: String,
    category_id: Option<Uint128>,
    dao_config: TeamDaoConfig,
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
    env: Env,
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

    let response = Response::new()
        .add_attribute("action", "update_entry_status")
        .add_attribute("entry_id", entry_id.to_string());

    // Create the team if we entered the created state
    if matches!(entry.status, EntryStatus::Created) {
        let dao_config = entry.dao_config;
        let id = entry_id;
        let TeamDaoConfig {
            dao_config,
            cw4_group_code_id,
        } = dao_config;

        let weight = 1000;
        let mut addrs = vec![];

        // Get approved applicants using the IndexedMap
        let mut members: Vec<Member> = applicants()
            .idx
            .entry_status
            .sub_prefix(entry_id)
            .range(deps.storage, None, None, Order::Ascending)
            .filter_map(|item| match item {
                Ok(((_entry_id, addr), status)) => {
                    if matches!(status, ApplicantStatus::Approved) {
                        addrs.push(addr.clone());
                        Some(Ok(Member {
                            addr: addr.to_string(),
                            weight,
                        }))
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(e)),
            })
            .collect::<StdResult<Vec<_>>>()?;

        // Add the creator as a member
        members.push(Member {
            addr: entry.creator.to_string(),
            weight,
        });
        addrs.push(entry.creator);

        // Generate predictable DAO address
        let dao_binding = format!("dao_{}{}{}", info.sender, env.block.height, id);
        let dao_salt: [u8; 32] = Sha256::digest(dao_binding.as_bytes()).into();
        let canonical_creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;
        let dao_code_info = deps.querier.query_wasm_code_info(dao_config.dao_code_id)?;
        let dao_canonical_addr = instantiate2_address(
            dao_code_info.checksum.as_slice(),
            &canonical_creator,
            &dao_salt,
        )
        .map_err(|e| StdError::generic_err(e.to_string()))?;
        let dao_addr = deps.api.addr_humanize(&dao_canonical_addr)?;

        // 1. Instantiate cw4-group voting module
        let voting_instantiate = ModuleInstantiateInfo {
            admin: Some(Admin::CoreModule {}),
            code_id: dao_config.cw4_voting_code_id,
            label: format!("Cw4Voting_{}", id),
            msg: to_json_binary(&dao_voting_cw4::msg::InstantiateMsg {
                group_contract: dao_voting_cw4::msg::GroupContract::New {
                    cw4_group_code_id,
                    cw4_group_salt: None,
                    initial_members: members,
                },
            })?,
            funds: None,
            salt: None,
        };

        // 2. Instantiate proposal module
        let proposal_instantiate = ModuleInstantiateInfo {
            admin: Some(Admin::CoreModule {}),
            code_id: dao_config.proposal_single_code_id,
            label: format!("ProposalSingle_{}", id),
            msg: to_json_binary(&dao_proposal_single::msg::InstantiateMsg {
                min_voting_period: None,
                max_voting_period: dao_config.max_voting_period,
                only_members_execute: true,
                allow_revoting: false,
                threshold: dao_config.threshold.clone(),
                pre_propose_info: dao_voting::pre_propose::PreProposeInfo::ModuleMayPropose {
                    info: ModuleInstantiateInfo {
                        code_id: dao_config.prepropose_single_code_id,
                        msg: to_json_binary(&dao_pre_propose_single::InstantiateMsg {
                            deposit_info: None,
                            submission_policy:
                                dao_voting::pre_propose::PreProposeSubmissionPolicy::Specific {
                                    dao_members: true,
                                    allowlist: vec![],
                                    denylist: vec![],
                                },
                            extension: Empty {},
                        })?,
                        admin: Some(Admin::CoreModule {}),
                        funds: None,
                        salt: None,
                        label: format!("PreProposeSingle_{}", id),
                    },
                },
                close_proposal_on_execution_failure: true,
                veto: None,
                delegation_module: None,
            })?,
            funds: None,
            salt: None,
        };

        // 3. Instantiate DAO core
        let dao_instantiate = WasmMsg::Instantiate2 {
            admin: Some(dao_addr.to_string()),
            code_id: dao_config.dao_code_id,
            label: format!("dao_{}", id),
            msg: to_json_binary(&dao_interface::msg::InstantiateMsg {
                admin: None,
                name: dao_config.dao_name,
                description: dao_config.dao_description,
                automatically_add_cw20s: true,
                automatically_add_cw721s: true,
                voting_module_instantiate_info: voting_instantiate,
                proposal_modules_instantiate_info: vec![proposal_instantiate],
                image_url: dao_config.image_url,
                initial_items: None,
                dao_uri: None,
                initial_actions: None,
            })?,
            funds: vec![],
            salt: dao_salt.into(),
        };

        for addr in addrs {
            USER_TEAMS.save(deps.storage, (&addr, &dao_addr), &())?;
        }

        Ok(response
            .add_attribute("dao", dao_addr)
            .add_message(dao_instantiate))
    } else {
        Ok(response)
    }
}

/// Allows a user to apply to a team entry
pub fn apply(deps: DepsMut, _env: Env, info: MessageInfo, entry_id: u64) -> StdResult<Response> {
    // Ensure the entry exists
    let entries = team_entries();
    let entry = entries
        .load(deps.storage, entry_id)
        .map_err(|_| StdError::generic_err("Team entry does not exist"))?;

    if info.sender == entry.creator {
        return Err(StdError::generic_err(
            "The creator is already a member of the enrollment",
        ));
    }
    // Validate the entry is open for applications
    if entry.status != crate::state::EntryStatus::Open {
        return Err(StdError::generic_err("Entry is not open for applications"));
    }

    // Ensure the applicant has not already applied
    let applicant_key = (entry_id, &info.sender);
    if applicants().has(deps.storage, applicant_key) {
        return Err(StdError::generic_err(
            "You have already applied to this entry",
        ));
    }

    // Save the applicant with Default status
    applicants().save(deps.storage, applicant_key, &ApplicantStatus::Default)?;

    // Update the count for default status
    let count_key = (entry_id, ApplicantStatus::Default.as_str());
    APPLICANTS_COUNT.update(deps.storage, count_key, |x| -> StdResult<u64> {
        x.unwrap_or_default()
            .checked_add(1)
            .ok_or_else(|| StdError::generic_err("Applicant count overflow"))
    })?;

    Ok(Response::new()
        .add_attribute("action", "apply")
        .add_attribute("applicant", info.sender)
        .add_attribute("entry_id", entry_id.to_string()))
}

pub fn withdraw_application(
    deps: DepsMut,
    info: MessageInfo,
    entry_id: u64,
) -> StdResult<Response> {
    let key = (entry_id, &info.sender);

    let status = applicants()
        .may_load(deps.storage, key)?
        .ok_or_else(|| StdError::not_found("Application"))?;

    if matches!(status, ApplicantStatus::Rejected { .. }) {
        return Err(StdError::generic_err(
            "Rejected applications cannot be withdrawn",
        ));
    }

    // Remove the applicant
    applicants().remove(deps.storage, key)?;

    // Update the count for the current status
    let count_key = (entry_id, status.as_str());
    APPLICANTS_COUNT.update(deps.storage, count_key, |x| -> StdResult<u64> {
        x.unwrap_or_default()
            .checked_sub(1)
            .ok_or_else(|| StdError::generic_err("Applicant count underflow"))
    })?;

    Ok(Response::new()
        .add_attribute("action", "withdraw_application")
        .add_attribute("entry_id", entry_id.to_string())
        .add_attribute("applicant", info.sender))
}

pub fn update_applicant_status(
    deps: DepsMut,
    info: MessageInfo,
    entry_id: u64,
    applicant: String,
    new_status: ApplicantStatus,
) -> StdResult<Response> {
    let applicant = deps.api.addr_validate(&applicant)?;
    let entry = team_entries().load(deps.storage, entry_id)?;
    if entry.creator != info.sender {
        return Err(StdError::generic_err("Unauthorized"));
    }

    let key = (entry_id, &applicant);

    let current_status = applicants()
        .may_load(deps.storage, key)?
        .ok_or_else(|| StdError::not_found("Application"))?;

    current_status.validate_transition(&new_status)?;

    // Update counts - decrement old status, increment new status
    let old_count_key = (entry_id, current_status.as_str());
    let new_count_key = (entry_id, new_status.as_str());

    // Decrement old status count
    APPLICANTS_COUNT.update(deps.storage, old_count_key, |x| -> StdResult<u64> {
        x.unwrap_or_default()
            .checked_sub(1)
            .ok_or_else(|| StdError::generic_err("Applicant count underflow"))
    })?;

    // Increment new status count
    APPLICANTS_COUNT.update(deps.storage, new_count_key, |x| -> StdResult<u64> {
        x.unwrap_or_default()
            .checked_add(1)
            .ok_or_else(|| StdError::generic_err("Applicant count overflow"))
    })?;

    // Update the applicant status
    applicants().save(deps.storage, key, &new_status)?;

    Ok(Response::new()
        .add_attribute("action", "update_applicant_status")
        .add_attribute("entry_id", entry_id.to_string())
        .add_attribute("applicant", applicant.to_string())
        .add_attribute("new_status", new_status.to_string()))
}
