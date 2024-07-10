use arena_interface::{
    competition::msg::EscrowInstantiateInfo,
    core::{CompetitionModuleQuery, CompetitionModuleResponse},
};
use arena_league_module::msg::LeagueInstantiateExt;
use arena_tournament_module::{msg::TournamentInstantiateExt, state::EliminationType};
use arena_wager_module::msg::WagerInstantiateExt;
use cosmwasm_std::{
    ensure, to_json_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, Empty, Env, MessageInfo,
    Order, Response, StdError, StdResult, SubMsg, Uint128, Uint64, WasmMsg,
};
use cw_balance::{BalanceUnchecked, MemberBalanceUnchecked};
use cw_utils::{must_pay, Expiration};

use crate::{
    msg::CompetitionInfoMsg,
    state::{
        enrollment_entries, CompetitionInfo, CompetitionType, EnrollmentEntry, EnrollmentInfo,
        ENROLLMENT_COUNT, ENROLLMENT_MEMBERS, ENROLLMENT_MEMBERS_COUNT, TEMP_ENROLLMENT_INFO,
    },
    ContractError,
};

pub const TRIGGER_COMPETITION_REPLY_ID: u64 = 1;

#[allow(clippy::too_many_arguments)]
pub fn create_enrollment(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    min_members: Option<Uint64>,
    max_members: Uint64,
    entry_fee: Option<Coin>,
    expiration: Expiration,
    category_id: Option<Uint128>,
    competition_info: CompetitionInfoMsg,
    competition_type: CompetitionType,
) -> Result<Response, ContractError> {
    ensure!(
        !expiration.is_expired(&env.block),
        ContractError::StdError(StdError::generic_err(
            "Cannot create an expired competition enrollment"
        ))
    );
    ensure!(
        expiration < competition_info.expiration,
        ContractError::StdError(StdError::generic_err(
            "Cannot have an enrollment with expiration before the competition's expiration"
        ))
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

    // Validate additional layered fees before saving
    if let Some(additional_layered_fees) = &competition_info.additional_layered_fees {
        additional_layered_fees
            .iter()
            .map(|x| x.into_checked(deps.as_ref()))
            .collect::<StdResult<Vec<_>>>()?;
    }

    let competition_id = ENROLLMENT_COUNT.load(deps.storage)? + Uint128::one();

    enrollment_entries().save(
        deps.storage,
        competition_id.u128(),
        &EnrollmentEntry {
            min_members,
            max_members,
            entry_fee,
            expiration,
            has_triggered_expiration: false,
            competition_info: CompetitionInfo::Pending {
                name: competition_info.name,
                description: competition_info.description,
                expiration: competition_info.expiration,
                rules: competition_info.rules,
                rulesets: competition_info.rulesets,
                banner: competition_info.banner,
                additional_layered_fees: competition_info.additional_layered_fees,
            },
            competition_type,
            host: info.sender,
            category_id,
            competition_module,
        },
    )?;

    ENROLLMENT_COUNT.save(deps.storage, &competition_id)?;

    Ok(Response::new()
        .add_attribute("action", "create_enrollment")
        .add_attribute("id", competition_id))
}

pub fn trigger_expiration(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: Uint128,
    escrow_id: u64,
) -> Result<Response, ContractError> {
    let entry = enrollment_entries().load(deps.storage, id.u128())?;

    ensure!(entry.host == info.sender, ContractError::Unauthorized {});
    ensure!(
        !entry.has_triggered_expiration,
        ContractError::StdError(StdError::generic_err(
            "Competition creation has already been triggered"
        ))
    );

    let members_count = ENROLLMENT_MEMBERS_COUNT.load(deps.storage, id.u128())?;

    // Check if we have met the minimum number of members
    let min_min_members = get_min_min_members(&entry.competition_type);
    let min_members = entry.min_members.unwrap_or(min_min_members);
    let is_expired = entry.expiration.is_expired(&env.block);

    if members_count < min_members && is_expired {
        // Set has_triggered_expiration to true and save the entry
        let new_data = EnrollmentEntry {
            has_triggered_expiration: true,
            ..entry.clone()
        };
        enrollment_entries().replace(deps.storage, id.u128(), Some(&new_data), Some(&entry))?;

        // Return a response indicating the enrollment was expired due to insufficient members
        return Ok(Response::new()
            .add_attribute("action", "trigger_expiration")
            .add_attribute("result", "expired_insufficient_members")
            .add_attribute("id", id.to_string())
            .add_attribute("required_members", min_members.to_string())
            .add_attribute("actual_members", members_count.to_string()));
    }

    ensure!(
        entry.max_members == members_count || is_expired,
        ContractError::TriggerFailed {
            max_members: entry.max_members,
            current_members: members_count,
            expiration: entry.expiration
        }
    );

    // TODO: optimize this to handle a huge amount
    let members = ENROLLMENT_MEMBERS
        .prefix(id.u128())
        .range(deps.storage, None, None, Order::Descending)
        .map(|x| x.map(|y| y.0))
        .collect::<StdResult<Vec<_>>>()?;

    let mut enrollment_info = EnrollmentInfo {
        enrollment_id: id.u128(),
        module_addr: entry.competition_module.clone(),
        amount: None,
    };

    let creation_msg = match entry.competition_info.clone() {
        CompetitionInfo::Pending {
            name,
            description,
            expiration,
            rules,
            rulesets,
            banner,
            additional_layered_fees,
        } => Ok({
            let escrow = if let Some(entry_fee) = &entry.entry_fee {
                let members_count = ENROLLMENT_MEMBERS_COUNT.load(deps.storage, id.u128())?;
                let total = Coin {
                    denom: entry_fee.denom.clone(),
                    amount: entry_fee.amount.checked_mul(members_count.into())?,
                };

                enrollment_info.amount = Some(total.clone());

                Some(EscrowInstantiateInfo {
                    code_id: escrow_id,
                    msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                        dues: vec![MemberBalanceUnchecked {
                            addr: env.contract.address.to_string(),
                            balance: BalanceUnchecked {
                                native: Some(vec![total]),
                                cw20: None,
                                cw721: None,
                            },
                        }],
                        should_activate_on_funded: Some(false),
                    })?,
                    label: "Arena Escrow".to_string(),
                    additional_layered_fees,
                })
            } else {
                None
            };

            match entry.competition_type.clone() {
                CompetitionType::Wager {} => {
                    let registered_members = if members.len() == 2 {
                        Some(members.iter().map(|x| x.to_string()).collect())
                    } else {
                        None
                    };
                    to_json_binary(&arena_wager_module::msg::ExecuteMsg::CreateCompetition {
                        host: Some(entry.host.to_string()),
                        category_id: entry.category_id,
                        escrow,
                        name,
                        description,
                        expiration,
                        rules,
                        rulesets,
                        banner,
                        should_activate_on_funded: Some(false),
                        instantiate_extension: WagerInstantiateExt { registered_members },
                    })?
                }
                CompetitionType::League {
                    match_win_points,
                    match_draw_points,
                    match_lose_points,
                    distribution,
                } => to_json_binary(&arena_league_module::msg::ExecuteMsg::CreateCompetition {
                    host: Some(entry.host.to_string()),
                    category_id: entry.category_id,
                    escrow,
                    name,
                    description,
                    expiration,
                    rules,
                    rulesets,
                    banner,
                    should_activate_on_funded: Some(false),
                    instantiate_extension: LeagueInstantiateExt {
                        match_win_points,
                        match_draw_points,
                        match_lose_points,
                        teams: members.iter().map(|x| x.to_string()).collect(),
                        distribution,
                    },
                })?,
                CompetitionType::Tournament {
                    elimination_type,
                    distribution,
                } => to_json_binary(
                    &arena_tournament_module::msg::ExecuteMsg::CreateCompetition {
                        host: Some(entry.host.to_string()),
                        category_id: entry.category_id,
                        escrow,
                        name,
                        description,
                        expiration,
                        rules,
                        rulesets,
                        banner,
                        should_activate_on_funded: Some(false),
                        instantiate_extension: TournamentInstantiateExt {
                            elimination_type,
                            teams: members.iter().map(|x| x.to_string()).collect(),
                            distribution,
                        },
                    },
                )?,
            }
        }),
        _ => Err(ContractError::StdError(StdError::generic_err(
            "The competition has already been generated",
        ))),
    }?;

    let sub_msg = SubMsg::reply_always(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: entry.competition_module.to_string(),
            msg: creation_msg,
            funds: vec![],
        }),
        TRIGGER_COMPETITION_REPLY_ID,
    );

    TEMP_ENROLLMENT_INFO.save(deps.storage, &enrollment_info)?;

    Ok(Response::new()
        .add_attribute("action", "trigger_expiration")
        .add_attribute("competition_module", enrollment_info.module_addr)
        .add_attribute("id", id.to_string())
        .add_attribute(
            "amount",
            enrollment_info
                .amount
                .map(|x| x.to_string())
                .unwrap_or("None".to_owned()),
        )
        .add_submessage(sub_msg))
}

pub fn enroll(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: Uint128,
) -> Result<Response, ContractError> {
    ensure!(
        !ENROLLMENT_MEMBERS.has(deps.storage, (id.u128(), &info.sender)),
        ContractError::AlreadyEnrolled {}
    );
    let entry = enrollment_entries().load(deps.storage, id.u128())?;

    ensure!(
        !entry.has_triggered_expiration,
        ContractError::StdError(StdError::generic_err(
            "Competition has already been generated"
        ))
    );
    if let Some(entry_fee) = &entry.entry_fee {
        let paid_amount = must_pay(&info, &entry_fee.denom)?;

        ensure!(
            paid_amount == entry_fee.amount,
            ContractError::EntryFeeNotPaid {
                fee: entry_fee.amount
            }
        );
    }

    let members_count = ENROLLMENT_MEMBERS_COUNT.update(
        deps.storage,
        id.u128(),
        |x| -> Result<_, ContractError> {
            let members_count = x.unwrap_or_default();

            ensure!(
                members_count < entry.max_members,
                StdError::generic_err("Competition is at membership capacity")
            );

            Ok(members_count.checked_add(Uint64::one())?)
        },
    )?;
    ENROLLMENT_MEMBERS.save(deps.storage, (id.u128(), &info.sender), &Empty {})?;

    Ok(Response::new()
        .add_attribute("action", "enroll")
        .add_attribute("members_count", members_count.to_string()))
}

pub fn withdraw(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: Uint128,
) -> Result<Response, ContractError> {
    // Check if the user is enrolled
    ensure!(
        ENROLLMENT_MEMBERS.has(deps.storage, (id.u128(), &info.sender)),
        ContractError::NotEnrolled {}
    );

    // Load the enrollment entry
    let entry = enrollment_entries().load(deps.storage, id.u128())?;

    // Check if the competition is still in Pending state
    let is_pending = matches!(entry.competition_info, CompetitionInfo::Pending { .. });

    // Ensure the competition hasn't been triggered yet and is still in Pending state
    ensure!(
        !entry.has_triggered_expiration || is_pending,
        ContractError::StdError(StdError::generic_err(
            "Enrollment has already been expired or competition has been created, withdrawal not possible"
        ))
    );

    // Remove the member from the enrollment
    ENROLLMENT_MEMBERS.remove(deps.storage, (id.u128(), &info.sender));

    // Update the members count
    let members_count =
        ENROLLMENT_MEMBERS_COUNT.update(deps.storage, id.u128(), |count| -> StdResult<_> {
            Ok(count.unwrap_or_default().checked_sub(Uint64::one())?)
        })?;

    // Prepare the refund if there was an entry fee
    let refund_msg = if let Some(entry_fee) = &entry.entry_fee {
        vec![CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![entry_fee.clone()],
        })]
    } else {
        vec![]
    };

    Ok(Response::new()
        .add_messages(refund_msg)
        .add_attribute("action", "withdraw")
        .add_attribute("id", id.to_string())
        .add_attribute("withdrawing_member", info.sender)
        .add_attribute("remaining_members", members_count.to_string()))
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
                play_third_place_match: _,
            } => Uint64::new(std::cmp::max(4, distribution.len()) as u64),
            EliminationType::DoubleElimination => {
                Uint64::new(std::cmp::max(3, distribution.len()) as u64)
            }
        },
    }
}
