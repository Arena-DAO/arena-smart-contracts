use arena_interface::{
    competition::{
        msg::EscrowContractInfo,
        types::{CompetitionType, DaoConfig, EliminationType},
    },
    core::{CompetitionModuleQuery, CompetitionModuleResponse},
    escrow::{self},
    fees::FeeInformation,
    group::{self, GroupContractInfo, MemberMsg},
    helpers::is_expired,
};
use arena_league_module::msg::LeagueInstantiateExt;
use arena_tournament_module::msg::TournamentInstantiateExt;
use arena_wager_module::msg::WagerInstantiateExt;
use cosmwasm_std::{
    ensure, instantiate2_address, to_json_binary, Addr, Attribute, BlockInfo, Coin, CosmosMsg,
    DepsMut, Empty, Env, MessageInfo, Response, StdError, StdResult, SubMsg, Timestamp, Uint128,
    Uint64, WasmMsg,
};
use cw_utils::must_pay;
use dao_interface::{
    state::{Admin, ModuleInstantiateInfo},
    voting::VotingPowerAtHeightResponse,
};
use itertools::Itertools as _;
use sha2::{Digest, Sha256};

use crate::{
    msg::CompetitionInfoMsg,
    state::{
        enrollment_entries, CompetitionInfo, EnrollmentEntry, EnrollmentInfo, ENROLLMENT_COUNT,
        TEMP_ENROLLMENT_INFO,
    },
    ContractError,
};

pub const FINALIZE_COMPETITION_REPLY_ID: u64 = 1;
/// Team size is limited to cw4-group's max size limit
const TEAM_SIZE_LIMIT: u32 = 30;

pub(crate) fn is_enrollment_expired(
    current: &BlockInfo,
    date: &Timestamp,
    duration_before: u64,
) -> bool {
    current.time > date.minus_seconds(duration_before)
}

#[allow(clippy::too_many_arguments)]
pub fn create_enrollment(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    min_members: Option<Uint64>,
    max_members: Uint64,
    entry_fee: Option<Coin>,
    duration_before: u64,
    category_id: Option<Uint128>,
    competition_info: CompetitionInfoMsg,
    competition_type: CompetitionType,
    group_contract_info: ModuleInstantiateInfo,
    required_team_size: Option<u32>,
    escrow_contract_info: EscrowContractInfo,
    use_dao_host: Option<DaoConfig>,
) -> Result<Response, ContractError> {
    ensure!(
        !is_expired(
            &env.block,
            &competition_info.date,
            competition_info.duration
        ),
        ContractError::StdError(StdError::generic_err(
            "Cannot create an expired competition enrollment"
        ))
    );
    ensure!(
        !is_enrollment_expired(
            &env.block,
            &competition_info.date,
            competition_info.duration
        ),
        ContractError::StdError(StdError::generic_err("Cannot create expired enrollment"))
    );

    let min_min_members = get_min_min_members(&competition_type);
    if let Some(min_members) = min_members {
        ensure!(
            min_members <= max_members,
            ContractError::StdError(StdError::generic_err(
                "Min members cannot be larger than max members"
            ))
        );
        ensure!(
            min_members >= min_min_members,
            ContractError::StdError(StdError::generic_err(format!(
                "Min members cannot be less than the required minimum of {}",
                min_min_members
            )))
        )
    } else {
        ensure!(
            min_min_members <= max_members,
            ContractError::StdError(StdError::generic_err(
                "Max members must be at least the required minimum number of members"
            ))
        );
    }

    // Validate category
    let ownership = cw_ownable::get_ownership(deps.storage)?;
    let competition_module = if let Some(owner) = ownership.owner {
        if let Some(category_id) = category_id {
            if let Some(rulesets) = &competition_info.rulesets {
                ensure!(
                    deps.querier.query_wasm_smart::<bool>(
                        &owner,
                        &arena_interface::core::QueryMsg::QueryExtension {
                            msg: arena_interface::core::QueryExt::IsValidCategoryAndRulesets {
                                category_id,
                                rulesets: rulesets.clone(),
                            },
                        },
                    )?,
                    ContractError::StdError(StdError::generic_err(
                        "Invalid category and rulesets combination"
                    ))
                );
            }
        }

        let competition_module_response = deps
            .querier
            .query_wasm_smart::<Option<CompetitionModuleResponse<Addr>>>(
                owner,
                &arena_interface::core::QueryMsg::QueryExtension {
                    msg: arena_interface::core::QueryExt::CompetitionModule {
                        query: CompetitionModuleQuery::Key(competition_type.to_string(), None),
                    },
                },
            )?;

        if let Some(competition_module) = competition_module_response {
            ensure!(
                competition_module.is_enabled,
                ContractError::StdError(StdError::generic_err(
                    "Cannot use a disabled competition module"
                ))
            );

            Ok(competition_module.addr)
        } else {
            Err(ContractError::StdError(StdError::generic_err(
                "Could not find the competition module",
            )))
        }
    } else {
        Err(ContractError::OwnershipError(
            cw_ownable::OwnershipError::NoOwner,
        ))
    }?;

    let competition_id = ENROLLMENT_COUNT.update(deps.storage, |x| -> StdResult<_> {
        Ok(x.checked_add(Uint128::one())?)
    })?;

    let mut msgs = vec![];

    // Generate the group contract
    let binding = format!("{}{}{}", info.sender, env.block.height, competition_id);
    let salt: [u8; 32] = Sha256::digest(binding.as_bytes()).into();
    let canonical_creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;
    let code_info = deps
        .querier
        .query_wasm_code_info(group_contract_info.code_id)?;
    let canonical_addr = instantiate2_address(&code_info.checksum, &canonical_creator, &salt)?;

    msgs.push(CosmosMsg::Wasm(WasmMsg::Instantiate2 {
        admin: Some(env.contract.address.to_string()),
        code_id: group_contract_info.code_id,
        label: group_contract_info.label,
        msg: group_contract_info.msg,
        funds: vec![],
        salt: salt.into(),
    }));

    let group_contract = deps.api.addr_humanize(&canonical_addr)?;

    // Handle escrow setup
    let (escrow_addr, fees) = match escrow_contract_info {
        EscrowContractInfo::Existing {
            addr,
            additional_layered_fees,
        } => {
            let addr = deps.api.addr_validate(&addr)?;
            let fees = additional_layered_fees
                .map(|fees| {
                    fees.iter()
                        .map(|fee| fee.into_checked(deps.as_ref()))
                        .collect::<StdResult<Vec<_>>>()
                })
                .transpose()?;

            (addr, fees)
        }
        EscrowContractInfo::New {
            code_id,
            msg,
            label,
            additional_layered_fees,
        } => {
            let fees = additional_layered_fees
                .map(|fees| {
                    fees.iter()
                        .map(|fee| fee.into_checked(deps.as_ref()))
                        .collect::<StdResult<Vec<_>>>()
                })
                .transpose()?;

            let binding = format!("{}{}{}", info.sender, env.block.height, competition_id);
            let salt: [u8; 32] = Sha256::digest(binding.as_bytes()).into();
            let canonical_creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;
            let code_info = deps.querier.query_wasm_code_info(code_id)?;
            let canonical_addr =
                instantiate2_address(&code_info.checksum, &canonical_creator, &salt)?;

            msgs.push(CosmosMsg::Wasm(WasmMsg::Instantiate2 {
                admin: Some(env.contract.address.to_string()),
                code_id,
                label,
                msg,
                funds: vec![],
                salt: salt.into(),
            }));

            let escrow_addr = deps.api.addr_humanize(&canonical_addr)?;

            (escrow_addr, fees)
        }
    };

    enrollment_entries().save(
        deps.storage,
        competition_id.u128(),
        &EnrollmentEntry {
            min_members,
            max_members,
            entry_fee,
            duration_before,
            has_finalized: false,
            competition_info: CompetitionInfo::Pending {
                name: competition_info.name,
                description: competition_info.description,
                date: competition_info.date,
                duration: competition_info.duration,
                rules: competition_info.rules,
                rulesets: competition_info.rulesets,
                banner: competition_info.banner,
                additional_layered_fees: fees,
                escrow: escrow_addr,
                group_contract,
            },
            competition_type,
            host: info.sender,
            category_id,
            competition_module,
            required_team_size,
            use_dao_host,
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "create_enrollment")
        .add_attribute("id", competition_id)
        .add_messages(msgs))
}

pub fn finalize(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: Uint128,
) -> Result<Response, ContractError> {
    // Load enrollment entry from storage
    let enrollment = enrollment_entries().load(deps.storage, id.u128())?;

    // Authorization checks
    ensure!(
        enrollment.host == info.sender,
        ContractError::Unauthorized {}
    );
    ensure!(
        !enrollment.has_finalized,
        ContractError::AlreadyFinalized {}
    );

    // Get competition info reference and validate
    let (group_contract, escrow, date) = match &enrollment.competition_info {
        CompetitionInfo::Pending {
            escrow,
            group_contract,
            date,
            ..
        } => (group_contract, escrow, date),
        CompetitionInfo::Existing { .. } => return Err(ContractError::AlreadyFinalized {}),
    };

    // Query current member count
    let members_count: Uint64 = deps.querier.query_wasm_smart(
        group_contract.to_string(),
        &group::QueryMsg::Custom(group::CustomQueryMsg::MembersCount {}),
    )?;

    // Check member requirements and expiration
    let min_min_members = get_min_min_members(&enrollment.competition_type);
    let min_members = enrollment.min_members.unwrap_or(min_min_members);
    let is_expired = is_enrollment_expired(&env.block, date, enrollment.duration_before);

    // Create updated entry with finalized status
    let new_enrollment = EnrollmentEntry {
        has_finalized: true,
        ..enrollment.clone()
    };

    // Handle case when there are insufficient members and enrollment is expired
    if members_count < min_members && is_expired {
        enrollment_entries().replace(
            deps.storage,
            id.u128(),
            Some(&new_enrollment),
            Some(&enrollment),
        )?;

        // Unlock escrow
        let escrow_msg = WasmMsg::Execute {
            contract_addr: escrow.to_string(),
            msg: to_json_binary(&escrow::ExecuteMsg::Lock {
                value: false,
                transfer_ownership: None,
            })?,
            funds: vec![],
        };

        return Ok(Response::new()
            .add_attribute("action", "finalize")
            .add_attribute("result", "finalized_insufficient_members")
            .add_attribute("id", id.to_string())
            .add_attribute("required_members", min_members.to_string())
            .add_attribute("actual_members", members_count.to_string())
            .add_message(escrow_msg));
    }

    // Verify member count requirements are met
    ensure!(
        enrollment.max_members == members_count || is_expired,
        ContractError::FinalizeFailed {
            max_members: enrollment.max_members,
            current_members: members_count,
        }
    );

    // DAO host
    let mut msgs = vec![];
    let host = if let Some(ref dao_config) = enrollment.use_dao_host {
        // Generate predictable DAO address
        let dao_binding = format!("dao_{}{}{}", info.sender, env.block.height, id);
        let dao_salt: [u8; 32] = Sha256::digest(dao_binding.as_bytes()).into();
        let canonical_creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;
        let dao_code_info = deps.querier.query_wasm_code_info(dao_config.dao_code_id)?;
        let dao_canonical_addr =
            instantiate2_address(&dao_code_info.checksum, &canonical_creator, &dao_salt)?;
        let dao_addr = deps.api.addr_humanize(&dao_canonical_addr)?;

        // 1. Instantiate cw4-group voting module
        let voting_instantiate = ModuleInstantiateInfo {
            admin: Some(Admin::CoreModule {}),
            code_id: dao_config.cw4_voting_code_id,
            label: format!("Cw4Voting_{}", id),
            msg: to_json_binary(&dao_voting_cw4::msg::InstantiateMsg {
                group_contract: dao_voting_cw4::msg::GroupContract::Existing {
                    address: group_contract.to_string(),
                },
            })?,
            funds: vec![],
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
                        funds: vec![],
                        label: format!("PreProposeSingle_{}", id),
                    },
                },
                close_proposal_on_execution_failure: true,
                veto: None,
            })?,
            funds: vec![],
        };

        // 3. Instantiate DAO core
        let dao_instantiate = WasmMsg::Instantiate2 {
            admin: Some(dao_addr.to_string()),
            code_id: dao_config.dao_code_id,
            label: format!("dao_{}", id),
            msg: to_json_binary(&dao_interface::msg::InstantiateMsg {
                admin: None,
                name: format!("Competition DAO {}", id),
                description: format!("DAO for competition {}", id),
                automatically_add_cw20s: false,
                automatically_add_cw721s: false,
                voting_module_instantiate_info: voting_instantiate,
                proposal_modules_instantiate_info: vec![proposal_instantiate],
                image_url: None,
                initial_items: None,
                dao_uri: None,
                initial_dao_actions: None,
            })?,
            funds: vec![],
            salt: dao_salt.into(),
        };

        msgs.push(CosmosMsg::Wasm(dao_instantiate));

        dao_addr
    } else {
        enrollment.host.clone()
    };

    // Create competition message based on competition type
    let creation_msg = match &enrollment.competition_info {
        CompetitionInfo::Pending {
            name,
            description,
            rules,
            rulesets,
            banner,
            additional_layered_fees,
            escrow,
            group_contract,
            date,
            duration,
        } => {
            // Process additional fee information
            let additional_layered_fees = additional_layered_fees.as_ref().map(|fees| {
                fees.iter()
                    .map(|fee| FeeInformation {
                        tax: fee.tax,
                        receiver: fee.receiver.to_string(),
                        cw20_msg: fee.cw20_msg.clone(),
                        cw721_msg: fee.cw721_msg.clone(),
                    })
                    .collect_vec()
            });

            // Prepare contract information
            let escrow_info = EscrowContractInfo::Existing {
                addr: escrow.to_string(),
                additional_layered_fees,
            };

            let group_info = GroupContractInfo::Existing {
                addr: group_contract.to_string(),
            };

            // Create appropriate competition message based on type
            match &enrollment.competition_type {
                CompetitionType::Wager {} => {
                    to_json_binary(&arena_wager_module::msg::ExecuteMsg::CreateCompetition {
                        host: Some(host.to_string()),
                        category_id: enrollment.category_id,
                        escrow: escrow_info.clone(),
                        name: name.clone(),
                        description: description.clone(),
                        date: *date,
                        duration: *duration,
                        rules: rules.clone(),
                        rulesets: rulesets.clone(),
                        banner: banner.clone(),
                        instantiate_extension: WagerInstantiateExt {},
                        group_contract: group_info.clone(),
                    })?
                }
                CompetitionType::League {
                    match_win_points,
                    match_draw_points,
                    match_lose_points,
                    distribution,
                } => to_json_binary(&arena_league_module::msg::ExecuteMsg::CreateCompetition {
                    host: Some(host.to_string()),
                    category_id: enrollment.category_id,
                    escrow: escrow_info.clone(),
                    name: name.clone(),
                    description: description.clone(),
                    date: *date,
                    duration: *duration,
                    rules: rules.clone(),
                    rulesets: rulesets.clone(),
                    banner: banner.clone(),
                    instantiate_extension: LeagueInstantiateExt {
                        match_win_points: *match_win_points,
                        match_draw_points: *match_draw_points,
                        match_lose_points: *match_lose_points,
                        distribution: distribution.clone(),
                    },
                    group_contract: group_info.clone(),
                })?,
                CompetitionType::Tournament {
                    elimination_type,
                    distribution,
                } => to_json_binary(
                    &arena_tournament_module::msg::ExecuteMsg::CreateCompetition {
                        host: Some(host.to_string()),
                        category_id: enrollment.category_id,
                        escrow: escrow_info.clone(),
                        name: name.clone(),
                        description: description.clone(),
                        date: *date,
                        duration: *duration,
                        rules: rules.clone(),
                        rulesets: rulesets.clone(),
                        banner: banner.clone(),
                        instantiate_extension: TournamentInstantiateExt {
                            elimination_type: *elimination_type,
                            distribution: distribution.clone(),
                        },
                        group_contract: group_info,
                    },
                )?,
            }
        }
        CompetitionInfo::Existing { .. } => return Err(ContractError::AlreadyFinalized {}),
    };

    // Save updated state
    enrollment_entries().replace(
        deps.storage,
        id.u128(),
        Some(&new_enrollment),
        Some(&enrollment),
    )?;
    TEMP_ENROLLMENT_INFO.save(
        deps.storage,
        &EnrollmentInfo {
            module_addr: enrollment.competition_module.clone(),
            enrollment_id: id.u128(),
            escrow_addr: escrow.clone(),
        },
    )?;

    // Prepare msg
    let submsg = SubMsg::reply_always(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: enrollment.competition_module.to_string(),
            msg: creation_msg,
            funds: vec![],
        }),
        FINALIZE_COMPETITION_REPLY_ID,
    );

    // Return response with competition creation message
    Ok(Response::new()
        .add_attribute("action", "finalize")
        .add_attribute("competition_module", enrollment.competition_module)
        .add_attribute("id", id.to_string())
        .add_messages(msgs)
        .add_submessage(submsg))
}

pub fn enroll(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: Uint128,
    team: Option<String>,
) -> Result<Response, ContractError> {
    let enrollment = enrollment_entries().load(deps.storage, id.u128())?;

    ensure!(
        !enrollment.has_finalized,
        ContractError::AlreadyFinalized {}
    );
    let (group_contract, escrow) = match &enrollment.competition_info {
        CompetitionInfo::Pending {
            escrow,
            group_contract,
            ..
        } => (group_contract, escrow),
        CompetitionInfo::Existing { .. } => return Err(ContractError::AlreadyFinalized {}),
    };

    let mut msgs = vec![];
    if let Some(entry_fee) = enrollment.entry_fee {
        let paid_amount = must_pay(&info, &entry_fee.denom)?;

        ensure!(
            paid_amount == entry_fee.amount,
            ContractError::EntryFeeNotPaid { entry_fee }
        );

        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: escrow.to_string(),
            msg: to_json_binary(&escrow::ExecuteMsg::ReceiveNative {})?,
            funds: vec![entry_fee],
        }));
    };

    let member_count: Uint64 = deps.querier.query_wasm_smart(
        group_contract.to_string(),
        &group::QueryMsg::Custom(group::CustomQueryMsg::MembersCount {}),
    )?;

    ensure!(
        member_count < enrollment.max_members,
        ContractError::EnrollmentMaxMembers {}
    );

    // Set correct member
    let member = if let Some(team) = team {
        let team = deps.api.addr_validate(&team)?;
        let voting_power_response: VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
            team.to_string(),
            &dao_interface::msg::QueryMsg::VotingPowerAtHeight {
                address: info.sender.to_string(),
                height: None,
            },
        )?;

        if voting_power_response.power.is_zero() {
            return Err(ContractError::NotTeamMember {});
        }

        team
    } else {
        info.sender
    };

    // Ensure team size requirement is handled
    if let Some(required_team_size) = enrollment.required_team_size {
        if required_team_size != 1 {
            deps.querier
                .query_wasm_contract_info(&member)
                .map_err(|_| ContractError::TeamSizeMismatch { required_team_size })?;
            let dao_voting_module: Addr = deps.querier.query_wasm_smart(
                member.to_string(),
                &dao_interface::msg::QueryMsg::VotingModule {},
            )?;
            let group_contract: Addr = deps.querier.query_wasm_smart(
                dao_voting_module,
                &dao_voting_cw4::msg::QueryMsg::GroupContract {},
            )?;
            let member_list_response: cw4::MemberListResponse = deps.querier.query_wasm_smart(
                group_contract,
                &cw4::Cw4QueryMsg::ListMembers {
                    start_after: None,
                    limit: Some(TEAM_SIZE_LIMIT),
                },
            )?;

            ensure!(
                member_list_response.members.len() as u32 == required_team_size,
                ContractError::TeamSizeMismatch { required_team_size }
            );
        }
    }

    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: group_contract.to_string(),
        msg: to_json_binary(&group::ExecuteMsg::UpdateMembers {
            to_add: Some(vec![group::AddMemberMsg {
                addr: member.to_string(),
                seed: None,
                power: Uint64::new(1000),
            }]),
            to_remove: None,
            to_update: None,
        })?,
        funds: vec![],
    }));

    Ok(Response::new()
        .add_attribute("action", "enroll")
        .add_messages(msgs))
}

pub fn withdraw(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: Uint128,
    team: Option<String>,
) -> Result<Response, ContractError> {
    // Load the enrollment entry
    let enrollment = enrollment_entries().load(deps.storage, id.u128())?;

    // Set correct member
    let member = if let Some(team) = team {
        let team = deps.api.addr_validate(&team)?;
        let voting_power_response: VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
            team.to_string(),
            &dao_interface::msg::QueryMsg::VotingPowerAtHeight {
                address: info.sender.to_string(),
                height: None,
            },
        )?;

        if voting_power_response.power.is_zero() {
            return Err(ContractError::NotTeamMember {});
        }

        team
    } else {
        info.sender
    };

    Ok(_withdraw(enrollment, vec![member.to_string()], id)?.add_attribute("action", "withdraw"))
}

pub fn force_withdraw(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: Uint128,
    members: Vec<String>,
) -> Result<Response, ContractError> {
    // Load the enrollment entry
    let enrollment = enrollment_entries().load(deps.storage, id.u128())?;

    ensure!(
        enrollment.host == info.sender,
        ContractError::Unauthorized {}
    );

    let members = members.into_iter().unique().collect::<Vec<_>>();

    ensure!(
        !members.is_empty(),
        ContractError::StdError(StdError::generic_err(
            "No members to force_withdraw provided"
        ))
    );

    Ok(_withdraw(enrollment, members, id)?.add_attribute("action", "force_withdraw"))
}

pub fn _withdraw(
    enrollment: EnrollmentEntry,
    members: Vec<String>,
    id: Uint128,
) -> Result<Response, ContractError> {
    // If created, then we cannot withdraw through here anymore
    let (group_contract, escrow) = match &enrollment.competition_info {
        CompetitionInfo::Pending {
            escrow,
            group_contract,
            ..
        } => (group_contract, escrow),
        _ => return Err(ContractError::AlreadyFinalized {}),
    };

    // If there's an entry fee, create refund messages for each member
    let mut msgs = if let Some(entry_fee) = &enrollment.entry_fee {
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: escrow.to_string(),
            msg: to_json_binary(&escrow::ExecuteMsg::EnrollmentWithdraw {
                addrs: members.clone(),
                entry_fee: entry_fee.clone(),
            })?,
            funds: vec![],
        })]
    } else {
        vec![]
    };

    // Create group update message to remove all members
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: group_contract.to_string(),
        msg: to_json_binary(&group::ExecuteMsg::UpdateMembers {
            to_add: None,
            to_update: None,
            to_remove: Some(members.clone()),
        })?,
        funds: vec![],
    }));

    // Create attributes for each withdrawn member
    let member_attributes = members
        .into_iter()
        .map(|member| Attribute {
            key: "withdrawn_member".to_string(),
            value: member.to_string(),
        })
        .collect::<Vec<_>>();

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("id", id.to_string())
        .add_attributes(member_attributes))
}

fn get_min_min_members(competition_type: &CompetitionType) -> Uint64 {
    match competition_type {
        CompetitionType::Wager {} => Uint64::new(2),
        CompetitionType::League { distribution, .. } => {
            Uint64::new(std::cmp::max(distribution.len(), 2) as u64)
        }
        CompetitionType::Tournament {
            elimination_type,
            distribution,
        } => match elimination_type {
            EliminationType::SingleElimination {
                play_third_place_match,
            } => Uint64::new(std::cmp::max(
                if *play_third_place_match { 4 } else { 3 },
                distribution.len(),
            ) as u64),
            EliminationType::DoubleElimination => {
                Uint64::new(std::cmp::max(3, distribution.len()) as u64)
            }
        },
    }
}

pub fn set_rankings(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: Uint128,
    rankings: Vec<MemberMsg<String>>,
) -> Result<Response, ContractError> {
    let enrollment = enrollment_entries().load(deps.storage, id.u128())?;

    ensure!(
        enrollment.host == info.sender,
        ContractError::Unauthorized {}
    );
    let group_contract = match &enrollment.competition_info {
        CompetitionInfo::Pending { group_contract, .. } => group_contract,
        CompetitionInfo::Existing { .. } => return Err(ContractError::AlreadyFinalized {}),
    };

    let msg = WasmMsg::Execute {
        contract_addr: group_contract.to_string(),
        msg: to_json_binary(&group::ExecuteMsg::UpdateMembers {
            to_add: None,
            to_update: Some(rankings),
            to_remove: None,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_attribute("action", "set_rankings")
        .add_message(msg))
}
