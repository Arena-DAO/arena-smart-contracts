use std::collections::HashMap;

use arena_interface::{
    core::{
        CompetitionCategory, EditCompetitionCategory, NewCompetitionCategory, NewRuleset,
        PrePropose, ProposeMessage, ProposeMessages, Ruleset,
    },
    ratings::MemberResult,
};
use cosmwasm_std::{
    ensure, ensure_eq, ensure_ne, to_json_binary, Addr, Attribute, CosmosMsg, Decimal, Deps,
    DepsMut, Empty, Env, MessageInfo, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw_utils::Duration;
use dao_interface::state::ModuleInstantiateInfo;
use dao_pre_propose_base::error::PreProposeError;
use dao_voting::{
    pre_propose::PreProposeSubmissionPolicy,
    proposal::SingleChoiceProposeMsg,
    voting::{SingleChoiceAutoVote, Vote},
};

use crate::{
    state::{
        competition_categories, competition_modules, ratings, rulesets,
        COMPETITION_CATEGORIES_COUNT, ENROLLMENT_MODULES, RATING_PERIOD, RULESETS_COUNT, TAX,
    },
    ContractError,
};

pub const COMPETITION_MODULE_REPLY_ID: u64 = 1;
pub const DAO_REPLY_ID: u64 = 2;
pub const ESCROW_REPLY_ID: u64 = 3;
pub const COMPETITION_REPLY_ID: u64 = 5;

pub fn update_competition_modules(
    deps: DepsMut,
    sender: Addr,
    to_add: Option<Vec<ModuleInstantiateInfo>>,
    to_disable: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    // Disable specified competition modules
    if let Some(to_disable) = to_disable {
        for module_addr in &to_disable {
            let addr = deps.api.addr_validate(module_addr)?;
            competition_modules().update(
                deps.storage,
                &addr,
                |maybe_module| -> Result<_, ContractError> {
                    let mut module =
                        maybe_module.ok_or(ContractError::CompetitionModuleDoesNotExist {
                            addr: addr.clone(),
                        })?;
                    module.is_enabled = false;
                    Ok(module)
                },
            )?;
        }
    }

    // Convert new modules into wasm messages and prepare for instantiation
    let competition_module_msgs: Vec<SubMsg> = if let Some(to_add) = to_add {
        to_add
            .into_iter()
            .map(|info| info.into_wasm_msg(sender.clone()))
            .map(|wasm| SubMsg::reply_on_success(wasm, COMPETITION_MODULE_REPLY_ID))
            .collect()
    } else {
        vec![]
    };

    Ok(Response::new()
        .add_attribute("action", "update_competition_modules")
        .add_submessages(competition_module_msgs))
}

pub fn update_tax(deps: DepsMut, env: &Env, tax: Decimal) -> Result<Response, ContractError> {
    if tax >= Decimal::one() {
        return Err(ContractError::StdError(StdError::GenericErr {
            msg: "The dao tax must be less than 100%.".to_string(),
        }));
    }

    TAX.save(deps.storage, &tax, env.block.height)?;

    Ok(Response::new()
        .add_attribute("action", "update_tax")
        .add_attribute("tax", tax.to_string()))
}

pub fn update_rulesets(
    deps: DepsMut,
    to_add: Option<Vec<NewRuleset>>,
    to_disable: Option<Vec<Uint128>>,
) -> Result<Response, ContractError> {
    // Disable specified rulesets
    if let Some(to_disable) = to_disable {
        for id in to_disable {
            rulesets().update(deps.storage, id.u128(), |maybe_ruleset| -> StdResult<_> {
                let mut ruleset = maybe_ruleset.ok_or(StdError::GenericErr {
                    msg: format!("Could not find a ruleset with the id {}", id),
                })?;
                ruleset.is_enabled = false;
                Ok(ruleset)
            })?;
        }
    }

    // Add new rulesets
    let mut attrs = vec![];
    if let Some(to_add) = to_add {
        let mut current_id = RULESETS_COUNT.load(deps.storage)?;
        for ruleset in to_add {
            if !competition_categories().has(deps.storage, ruleset.category_id.u128()) {
                return Err(ContractError::CompetitionCategoryDoesNotExist {
                    id: ruleset.category_id,
                });
            }

            current_id = current_id.checked_add(Uint128::one())?;

            let new_ruleset = Ruleset {
                category_id: ruleset.category_id,
                id: current_id,
                rules: ruleset.rules,
                description: ruleset.description,
                is_enabled: true,
            };
            rulesets().save(deps.storage, current_id.u128(), &new_ruleset)?;

            attrs.push(Attribute::new(
                new_ruleset.id,
                format!(
                    "Category {} - {}",
                    new_ruleset.category_id, new_ruleset.description
                ),
            ))
        }

        RULESETS_COUNT.save(deps.storage, &current_id)?;
        attrs.push(Attribute::new("ruleset_count", current_id));
    }

    Ok(Response::new()
        .add_attribute("action", "update_rulesets")
        .add_attributes(attrs))
}

pub fn check_can_submit(
    deps: Deps,
    who: &Addr,
    config: &dao_pre_propose_base::state::Config,
) -> Result<(), PreProposeError> {
    match &config.submission_policy {
        PreProposeSubmissionPolicy::Anyone { denylist } => {
            if !denylist.contains(who) {
                return Ok(());
            }
        }
        PreProposeSubmissionPolicy::Specific {
            dao_members,
            allowlist,
            denylist,
        } => {
            // denylist overrides all other settings
            if !denylist.contains(who) {
                // if on the allowlist, return early
                if allowlist.contains(who) {
                    return Ok(());
                }

                // check DAO membership only if not on the allowlist
                if *dao_members {
                    ensure_active_competition_module(deps, who)?;
                }
            }
        }
    }

    Ok(())
}

pub fn ensure_active_competition_module(deps: Deps, who: &Addr) -> Result<(), PreProposeError> {
    if !competition_modules().has(deps.storage, who) {
        return Err(PreProposeError::Unauthorized {});
    }

    if !competition_modules().load(deps.storage, who)?.is_enabled {
        return Err(PreProposeError::Unauthorized {});
    }

    Ok(())
}

pub fn propose(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ProposeMessage,
) -> Result<Response, PreProposeError> {
    let config = PrePropose::default().config.load(deps.storage)?;
    check_can_submit(deps.as_ref(), &info.sender, &config)?;
    let originator = deps.api.addr_validate(&msg.originator)?;

    let deposit_messages = if let Some(ref deposit_info) = config.deposit_info {
        deposit_info.check_native_deposit_paid(&info)?;
        deposit_info.get_take_deposit_messages(&originator, &env.contract.address)?
    } else {
        vec![]
    };

    let proposal_module = PrePropose::default().proposal_module.load(deps.storage)?;

    // Snapshot the deposit using the ID of the proposal that we
    // will create.
    let next_id = deps.querier.query_wasm_smart(
        &proposal_module,
        &dao_interface::proposal::Query::NextProposalId {},
    )?;
    PrePropose::default().deposits.save(
        deps.storage,
        next_id,
        &(config.deposit_info, originator.clone()),
    )?;

    // Validate distribution
    if let Some(distribution) = &msg.distribution {
        distribution.into_checked(deps.as_ref())?;
    }

    // Construct message
    let msg =
        ProposeMessages::Propose(SingleChoiceProposeMsg {
            title: msg.title,
            description: msg.description,
            vote: Some(SingleChoiceAutoVote {
                vote: Vote::Yes,
                rationale: None,
            }),
            msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: info.sender.to_string(),
                msg: to_json_binary(&arena_interface::competition::msg::ExecuteBase::<
                    Empty,
                    Empty,
                >::ProcessCompetition {
                    competition_id: msg.competition_id,
                    distribution: msg.distribution,
                })?,
                funds: vec![],
            })],
            proposer: Some(info.sender.to_string()),
        });

    let propose_message = WasmMsg::Execute {
        contract_addr: proposal_module.into_string(),
        msg: to_json_binary(&msg)?,
        funds: vec![],
    };

    let hooks_msgs = PrePropose::default()
        .proposal_submitted_hooks
        .prepare_hooks(deps.storage, |a| {
            let execute = WasmMsg::Execute {
                contract_addr: a.into_string(),
                msg: to_json_binary(&msg)?,
                funds: vec![],
            };
            Ok(SubMsg::new(execute))
        })?;

    Ok(Response::default()
        .add_attribute("method", "execute_propose")
        .add_attribute("originator", originator)
        // It's important that the propose message is
        // first. Otherwise, a hook receiver could create a
        // proposal before us and invalidate our `NextProposalId
        // {}` query.
        .add_message(propose_message)
        .add_submessages(hooks_msgs)
        .add_messages(deposit_messages))
}

pub fn update_categories(
    deps: DepsMut,
    to_add: Option<Vec<NewCompetitionCategory>>,
    to_edit: Option<Vec<EditCompetitionCategory>>,
) -> Result<Response, ContractError> {
    // Disable specified categories
    if let Some(to_edit) = to_edit {
        for action in to_edit {
            let id = match action {
                EditCompetitionCategory::Disable { category_id } => category_id,
                EditCompetitionCategory::Edit {
                    category_id,
                    name: _,
                } => category_id,
            };
            competition_categories().update(
                deps.storage,
                id.u128(),
                |maybe_category| -> Result<_, ContractError> {
                    let mut category = maybe_category
                        .ok_or(ContractError::CompetitionCategoryDoesNotExist { id })?;

                    match action {
                        EditCompetitionCategory::Disable { category_id: _ } => {
                            category.is_enabled = false
                        }
                        EditCompetitionCategory::Edit {
                            category_id: _,
                            name,
                        } => {
                            category.name = name;
                        }
                    };

                    Ok(category)
                },
            )?;
        }
    }

    // Add new categories
    let mut attrs = vec![];
    if let Some(to_add) = to_add {
        let mut current_id = COMPETITION_CATEGORIES_COUNT.load(deps.storage)?;
        for category in to_add {
            current_id = current_id.checked_add(Uint128::one())?;

            let new_category = CompetitionCategory {
                id: current_id,
                name: category.name,
                is_enabled: true,
            };
            competition_categories().save(deps.storage, current_id.u128(), &new_category)?;

            attrs.push(Attribute::new(new_category.id, new_category.name));
        }

        COMPETITION_CATEGORIES_COUNT.save(deps.storage, &current_id)?;
        attrs.push(Attribute::new("category_count", current_id));
    }

    Ok(Response::new()
        .add_attribute("action", "update_categories")
        .add_attributes(attrs))
}

pub fn adjust_ratings(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    category_id: Uint128,
    member_results: Vec<(MemberResult<String>, MemberResult<String>)>,
) -> Result<Response, ContractError> {
    // Validate authorization - this message should only be executed by the competition modules
    ensure_active_competition_module(deps.as_ref(), &info.sender)?;

    // Create a map to store the new ratings for attribute creation
    let mut new_ratings: HashMap<String, Decimal> = HashMap::new();
    for (member_result_1, member_result_2) in member_results {
        // Ensure different addresses
        ensure_ne!(
            member_result_1.addr,
            member_result_2.addr,
            ContractError::StdError(StdError::generic_err(
                "Rating adjustment must be between different addresses"
            ))
        );
        // Ensure results are between 0 and 1
        ensure!(
            member_result_1.result >= Decimal::zero() && member_result_1.result <= Decimal::one(),
            ContractError::StdError(StdError::generic_err("Result 1 must be between 0 and 1"))
        );
        ensure!(
            member_result_2.result >= Decimal::zero() && member_result_2.result <= Decimal::one(),
            ContractError::StdError(StdError::generic_err("Result 2 must be between 0 and 1"))
        );
        // Ensure the sum of results is 1
        ensure_eq!(
            member_result_1.result + member_result_2.result,
            Decimal::one(),
            ContractError::StdError(StdError::generic_err("The sum of results must be 1"))
        );

        let addr_1 = deps.api.addr_validate(&member_result_1.addr)?;
        let addr_2 = deps.api.addr_validate(&member_result_2.addr)?;

        let key_1 = (category_id.u128(), &addr_1);
        let key_2 = (category_id.u128(), &addr_2);

        let maybe_rating_1 = ratings().may_load(deps.storage, key_1)?;
        let maybe_rating_2 = ratings().may_load(deps.storage, key_2)?;

        let mut rating_1 = maybe_rating_1.clone().unwrap_or_default();
        let mut rating_2 = maybe_rating_2.clone().unwrap_or_default();

        // Calculate changes
        glicko_2::update_rating(
            &env,
            &mut rating_1,
            &mut rating_2,
            member_result_1.result,
            member_result_2.result,
            &RATING_PERIOD.load(deps.storage)?,
        );

        // Update values
        ratings().replace(
            deps.storage,
            key_1,
            Some(&rating_1),
            maybe_rating_1.as_ref(),
        )?;
        ratings().replace(
            deps.storage,
            key_2,
            Some(&rating_2),
            maybe_rating_2.as_ref(),
        )?;

        // Store the new ratings in the map
        new_ratings.insert(addr_1.to_string(), rating_1.value);
        new_ratings.insert(addr_2.to_string(), rating_2.value);
    }

    let attrs = new_ratings.into_iter().map(|(addr, value)| Attribute {
        key: addr,
        value: value.to_string(),
    });

    Ok(Response::new()
        .add_attribute("action", "adjust_ratings")
        .add_attributes(attrs))
}

pub fn update_rating_period(deps: DepsMut, period: Duration) -> Result<Response, ContractError> {
    let value = match &period {
        Duration::Height(height) => height,
        Duration::Time(seconds) => seconds,
    };

    if *value == 0 {
        return Err(ContractError::StdError(StdError::generic_err(
            "Cannot have a period of 0",
        )));
    }

    RATING_PERIOD.save(deps.storage, &period)?;

    Ok(Response::new()
        .add_attribute("action", "update_rating_period")
        .add_attribute("period", period.to_string()))
}

pub fn update_enrollment_modules(
    deps: DepsMut,
    to_add: Option<Vec<String>>,
    to_remove: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    // Add new modules
    if let Some(addresses_to_add) = to_add {
        for addr_str in addresses_to_add {
            let addr = deps.api.addr_validate(&addr_str)?;
            ENROLLMENT_MODULES.save(deps.storage, &addr, &Empty {})?;
        }
    }

    // Remove modules
    if let Some(addresses_to_remove) = to_remove {
        for addr_str in addresses_to_remove {
            let addr = deps.api.addr_validate(&addr_str)?;
            if ENROLLMENT_MODULES.has(deps.storage, &addr) {
                ENROLLMENT_MODULES.remove(deps.storage, &addr);
            } else {
                return Err(ContractError::StdError(StdError::generic_err(format!(
                    "Enrollment module {} is not registered",
                    addr
                ))));
            }
        }
    }

    Ok(Response::new().add_attribute("action", "update_enrollment_modules"))
}
