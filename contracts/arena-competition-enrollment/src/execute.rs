use arena_interface::{
    competition::msg::EscrowInstantiateInfo,
    core::{CompetitionModuleQuery, CompetitionModuleResponse},
    fees::FeeInformation,
};
use arena_tournament_module::state::EliminationType;
use arena_wager_module::msg::WagerInstantiateExt;
use cosmwasm_std::{
    ensure, to_json_binary, Addr, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Order, Response,
    StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw_balance::{BalanceUnchecked, MemberBalanceUnchecked};
use cw_utils::{must_pay, Expiration};

use crate::{
    msg::CompetitionInfoMsg,
    state::{
        enrollment_entries, CompetitionInfo, CompetitionType, EnrollmentEntry, ENROLLMENT_COUNT,
        ENROLLMENT_MEMBERS, TEMP_ENROLLMENT,
    },
    ContractError,
};

pub const TRIGGER_COMPETITION_REPLY_ID: u64 = 1;

#[allow(clippy::too_many_arguments)]
pub fn create_enrollment(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    min_members: Option<Uint128>,
    max_members: Uint128,
    entry_fee: Option<Coin>,
    expiration: Expiration,
    category_id: Option<Uint128>,
    competition_info: CompetitionInfoMsg,
    competition_type: CompetitionType,
    is_creator_member: Option<bool>,
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

    let min_min_members = Uint128::new(match &competition_info.competition_type {
        CompetitionType::Wager {} => 2,
        CompetitionType::League { distribution, .. } => std::cmp::max(distribution.len(), 2),
        CompetitionType::Tournament {
            elimination_type,
            distribution,
        } => match elimination_type {
            EliminationType::SingleElimination {
                play_third_place_match: _,
            } => std::cmp::max(4, distribution.len()),
            EliminationType::DoubleElimination => std::cmp::max(3, distribution.len()),
        },
    } as u128);
    if let Some(min_members) = min_members {
        ensure!(
            min_members < max_members,
            ContractError::StdError(StdError::generic_err(
                "Min members cannot be larger than max members"
            ))
        );
        ensure!(
            min_members > min_min_members,
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

    // Defaults to false
    let is_creator_member = is_creator_member.unwrap_or(false);

    if let Some(entry_fee) = &entry_fee {
        if is_creator_member {
            let paid_amount = must_pay(&info, &entry_fee.denom)?;

            ensure!(
                paid_amount == entry_fee.amount,
                ContractError::StdError(StdError::generic_err(
                    "You must send the entry fee if `is_creator_member` is true"
                ))
            );
        }
    }

    // Validate category
    let ownership = cw_ownable::get_ownership(deps.storage)?;
    let mut competition_module = Addr::unchecked("default");
    if let Some(owner) = ownership.owner {
        ensure!(
            deps.querier.query_wasm_smart::<bool>(
                &owner,
                &arena_interface::core::QueryMsg::QueryExtension {
                    msg: arena_interface::core::QueryExt::IsValidCategoryAndRulesets {
                        category_id,
                        rulesets: competition_info.rulesets.clone(),
                    },
                },
            )?,
            ContractError::StdError(StdError::generic_err(
                "Invalid category and rulesets combination"
            ))
        );

        let competition_module_response = deps
            .querier
            .query_wasm_smart::<CompetitionModuleResponse<Addr>>(
                owner,
                &arena_interface::core::QueryMsg::QueryExtension {
                    msg: arena_interface::core::QueryExt::CompetitionModule {
                        query: CompetitionModuleQuery::Key(
                            competition_info.competition_type.to_string(),
                            None,
                        ),
                    },
                },
            )?;

        ensure!(
            competition_module_response.is_enabled,
            ContractError::StdError(StdError::generic_err(
                "Cannot use a disabled competition module"
            ))
        );

        competition_module = competition_module_response.addr;
    } else {
        return Err(ContractError::OwnershipError(
            cw_ownable::OwnershipError::NoOwner,
        ));
    }

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

pub fn trigger_creation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: Uint128,
    escrow_id: u64,
) -> Result<Response, ContractError> {
    let entry = enrollment_entries().load(deps.storage, id.u128())?;

    // TODO: validate enrollment state

    // TODO: optimize this to handle a huge amount
    let members = ENROLLMENT_MEMBERS
        .prefix(id.u128())
        .range(deps.storage, None, None, Order::Descending)
        .map(|x| x.map(|y| y.0))
        .collect::<StdResult<Vec<_>>>()?;

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
                let total = deps
                    .querier
                    .query_balance(env.contract.address.to_string(), entry_fee.denom.clone())?;

                Some(EscrowInstantiateInfo {
                    code_id: escrow_id,
                    msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                        dues: vec![MemberBalanceUnchecked {
                            addr: env.contract.address.to_string(),
                            balance: BalanceUnchecked {
                                native: vec![total],
                                cw20: vec![],
                                cw721: vec![],
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

            match &entry.competition_type {
                CompetitionType::Wager {} => {
                    let registered_members = if members.len() == 2 {
                        Some(members.iter().map(|x| x.to_string()).collect())
                    } else {
                        None
                    };
                    to_json_binary(&arena_wager_module::msg::ExecuteMsg::CreateCompetition {
                        category_id: entry.category_id,
                        host: arena_interface::competition::msg::ModuleInfo::Existing {
                            addr: entry.host.to_string(),
                        },
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
                } => todo!(),
                CompetitionType::Tournament {
                    elimination_type,
                    distribution,
                } => todo!(),
            }
        }),
        _ => Err(ContractError::StdError(StdError::generic_err(
            "The competition has already been generated",
        ))),
    }?;

    let submsg = SubMsg::reply_always(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: entry.competition_module.to_string(),
            msg: creation_msg,
            funds: vec![],
        }),
        TRIGGER_COMPETITION_REPLY_ID,
    );

    TEMP_ENROLLMENT.save(deps.storage, &entry)?;

    Ok(Response::new()
        .add_attribute("action", "trigger_creation")
        .add_submessage(submsg))
}

pub fn enroll(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: Uint128,
) -> Result<Response, ContractError> {
    todo!()
}
