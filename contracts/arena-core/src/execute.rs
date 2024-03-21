use arena_core_interface::msg::{
    CompetitionCategory, EditCompetitionCategory, NewCompetitionCategory, NewRuleset, PrePropose,
    ProposeMessage, ProposeMessages, Ruleset,
};
use cosmwasm_std::{
    to_json_binary, Addr, CosmosMsg, Decimal, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use dao_interface::state::ModuleInstantiateInfo;
use dao_pre_propose_base::error::PreProposeError;
use dao_voting::proposal::SingleChoiceProposeMsg;

use crate::{
    state::{
        competition_categories, competition_modules, rulesets, COMPETITION_CATEGORIES_COUNT,
        RULESETS_COUNT, TAX,
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
    to_add: Vec<ModuleInstantiateInfo>,
    to_disable: Vec<String>,
) -> Result<Response, ContractError> {
    // Ensure sender is authorized
    if PrePropose::default().dao.load(deps.storage)? != sender {
        return Err(ContractError::Unauthorized {});
    }

    // Disable specified competition modules
    for module_addr in &to_disable {
        let addr = deps.api.addr_validate(module_addr)?;
        competition_modules().update(
            deps.storage,
            addr.clone(),
            |maybe_module| -> Result<_, ContractError> {
                let mut module =
                    maybe_module.ok_or(ContractError::CompetitionModuleDoesNotExist { addr })?;
                module.is_enabled = false;
                Ok(module)
            },
        )?;
    }

    // Convert new modules into wasm messages and prepare for instantiation
    let competition_module_msgs: Vec<SubMsg> = to_add
        .into_iter()
        .map(|info| info.into_wasm_msg(sender.clone()))
        .map(|wasm| SubMsg::reply_on_success(wasm, COMPETITION_MODULE_REPLY_ID))
        .collect();

    Ok(Response::new()
        .add_attribute("action", "update_competition_modules")
        .add_submessages(competition_module_msgs))
}

pub fn update_tax(
    deps: DepsMut,
    env: &Env,
    sender: Addr,
    tax: Decimal,
) -> Result<Response, ContractError> {
    if PrePropose::default().dao.load(deps.storage)? != sender {
        return Err(ContractError::Unauthorized {});
    }
    if tax > Decimal::one() {
        return Err(ContractError::StdError(StdError::GenericErr {
            msg: "The dao tax cannot be greater than 100%.".to_string(),
        }));
    }

    TAX.save(deps.storage, &tax, env.block.height)?;

    Ok(Response::new()
        .add_attribute("action", "update_tax")
        .add_attribute("tax", tax.to_string()))
}

pub fn update_rulesets(
    deps: DepsMut,
    sender: Addr,
    to_add: Vec<NewRuleset>,
    to_disable: Vec<Uint128>,
) -> Result<Response, ContractError> {
    // Ensure sender is authorized
    if PrePropose::default().dao.load(deps.storage)? != sender {
        return Err(ContractError::Unauthorized {});
    }

    // Disable specified rulesets
    for id in to_disable {
        rulesets().update(deps.storage, id.u128(), |maybe_ruleset| -> StdResult<_> {
            let mut ruleset = maybe_ruleset.ok_or(StdError::GenericErr {
                msg: format!("Could not find a ruleset with the id {}", id),
            })?;
            ruleset.is_enabled = false;
            Ok(ruleset)
        })?;
    }

    // Add new rulesets
    let mut current_id = RULESETS_COUNT.load(deps.storage)?;
    for ruleset in to_add {
        if let Some(category_id) = ruleset.category_id {
            if !competition_categories().has(deps.storage, category_id.u128()) {
                return Err(ContractError::CompetitionCategoryDoesNotExist { id: category_id });
            }
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
    }
    RULESETS_COUNT.save(deps.storage, &current_id)?;

    Ok(Response::new()
        .add_attribute("action", "update_rulesets")
        .add_attribute("ruleset_count", current_id))
}

pub fn check_can_submit(
    deps: Deps,
    who: Addr,
    config: &dao_pre_propose_base::state::Config,
) -> Result<(), PreProposeError> {
    if !config.open_proposal_submission {
        if !competition_modules().has(deps.storage, who.clone()) {
            return Err(PreProposeError::Unauthorized {});
        }

        if !competition_modules().load(deps.storage, who)?.is_enabled {
            return Err(PreProposeError::Unauthorized {});
        }
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
    check_can_submit(deps.as_ref(), info.sender.clone(), &config)?;

    let deposit_messages = if let Some(ref deposit_info) = config.deposit_info {
        deposit_info.check_native_deposit_paid(&info)?;
        deposit_info.get_take_deposit_messages(&info.sender, &env.contract.address)?
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
        &(config.deposit_info, info.sender.clone()),
    )?;

    // Validate distribution
    if let Some(distribution) = &msg.distribution {
        distribution.into_checked(deps.as_ref())?;
    }

    // Construct message
    let msg = ProposeMessages::Propose(SingleChoiceProposeMsg {
        title: msg.title,
        description: msg.description,
        msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: info.sender.to_string(),
            msg: to_json_binary(
                &cw_competition::msg::ExecuteBase::<Empty, Empty>::ProcessCompetition {
                    competition_id: msg.id,
                    distribution: msg.distribution,
                    tax_cw20_msg: msg.tax_cw20_msg,
                    tax_cw721_msg: msg.tax_cw721_msg,
                },
            )?,
            funds: vec![],
        })],
        proposer: Some(info.sender.to_string()),
    });

    let propose_messsage = WasmMsg::Execute {
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
        .add_attribute("sender", info.sender)
        // It's important that the propose message is
        // first. Otherwise, a hook receiver could create a
        // proposal before us and invalidate our `NextProposalId
        // {}` query.
        .add_message(propose_messsage)
        .add_submessages(hooks_msgs)
        .add_messages(deposit_messages))
}

pub fn update_categories(
    deps: DepsMut,
    sender: Addr,
    to_add: Vec<NewCompetitionCategory>,
    to_edit: Vec<EditCompetitionCategory>,
) -> Result<Response, ContractError> {
    // Ensure sender is authorized
    if PrePropose::default().dao.load(deps.storage)? != sender {
        return Err(ContractError::Unauthorized {});
    }

    // Disable specified categories
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
                let mut category =
                    maybe_category.ok_or(ContractError::CompetitionCategoryDoesNotExist { id })?;

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

    // Add new categories
    let mut current_id = COMPETITION_CATEGORIES_COUNT.load(deps.storage)?;
    for category in to_add {
        current_id = current_id.checked_add(Uint128::one())?;

        let new_category = CompetitionCategory {
            id: current_id,
            name: category.name,
            is_enabled: true,
        };
        competition_categories().save(deps.storage, current_id.u128(), &new_category)?;
    }
    COMPETITION_CATEGORIES_COUNT.save(deps.storage, &current_id)?;

    Ok(Response::new()
        .add_attribute("action", "update_categories")
        .add_attribute("category_count", current_id))
}
