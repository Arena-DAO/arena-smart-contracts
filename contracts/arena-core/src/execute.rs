use arena_core_interface::msg::{NewRuleset, PrePropose, ProposeMessage, ProposeMessages, Ruleset};
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Decimal, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    StdError, SubMsg, Uint128, WasmMsg,
};
use dao_interface::state::ModuleInstantiateInfo;
use dao_pre_propose_base::error::PreProposeError;
use dao_voting::proposal::SingleChoiceProposeMsg;

use crate::{
    state::{competition_modules, rulesets, KEYS, RULESET_COUNT, TAX},
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
    if PrePropose::default().dao.load(deps.storage)? != sender {
        return Err(ContractError::Unauthorized {});
    }

    for addr in to_disable {
        let addr = deps.api.addr_validate(&addr)?;
        let module = competition_modules().update(deps.storage, addr.clone(), |x| match x {
            Some(mut module) => {
                module.is_enabled = false;
                Ok(module)
            }
            None => Err(ContractError::CompetitionModuleDoesNotExist {}),
        })?;
        KEYS.remove(deps.storage, module.key);
    }
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
    if PrePropose::default().dao.load(deps.storage)? != sender {
        return Err(ContractError::Unauthorized {});
    }

    for id in to_disable {
        rulesets().update(deps.storage, id.u128(), |x| match x {
            Some(mut ruleset) => {
                ruleset.is_enabled = false;
                Ok(ruleset)
            }
            None => Err(StdError::GenericErr {
                msg: format!("Could not find a ruleset with the id {}", id),
            }),
        })?;
    }

    let mut id = RULESET_COUNT.may_load(deps.storage)?.unwrap_or_default();
    for ruleset in to_add {
        rulesets().save(
            deps.storage,
            id.u128(),
            &Ruleset {
                id,
                rules: ruleset.rules,
                description: ruleset.description,
                is_enabled: true,
            },
        )?;
        id = id.checked_add(Uint128::one())?;
    }
    RULESET_COUNT.save(deps.storage, &id)?;

    Ok(Response::new()
        .add_attribute("action", "update_rulesets")
        .add_attribute("ruleset_count", id))
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

    // Construct message
    let msg = ProposeMessages::Propose(SingleChoiceProposeMsg {
        title: msg.title,
        description: msg.description,
        msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: info.sender.to_string(),
            msg: to_binary(
                &cw_competition::msg::ExecuteBase::<Empty, Empty>::ProcessCompetition {
                    id: msg.id,
                    distribution: msg.distribution,
                },
            )?,
            funds: vec![],
        })],
        proposer: Some(info.sender.to_string()),
    });

    let propose_messsage = WasmMsg::Execute {
        contract_addr: proposal_module.into_string(),
        msg: to_binary(&msg)?,
        funds: vec![],
    };

    let hooks_msgs = PrePropose::default()
        .proposal_submitted_hooks
        .prepare_hooks(deps.storage, |a| {
            let execute = WasmMsg::Execute {
                contract_addr: a.into_string(),
                msg: to_binary(&msg)?,
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
