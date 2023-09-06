use cosmwasm_std::{Addr, Decimal, DepsMut, Env, Response, StdError, SubMsg, Uint128};
use cw_competition::proposal::create_competition_proposals;
use dao_interface::state::ModuleInstantiateInfo;

use crate::{
    msg::PrePropose,
    state::{competition_modules, rulesets, Ruleset, KEYS, RULESET_COUNT, TAX},
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
    to_add: Vec<Ruleset>,
    to_disable: Vec<Uint128>,
) -> Result<Response, ContractError> {
    if PrePropose::default().dao.load(deps.storage)? != sender {
        return Err(ContractError::Unauthorized {});
    }

    for id in to_disable {
        rulesets().update(deps.storage, id.u128(), |x| match x {
            Some(mut module) => {
                module.is_enabled = false;
                Ok(module)
            }
            None => Err(StdError::GenericErr {
                msg: format!("Could not find a competition module with the id {}", id),
            }),
        })?;
    }

    let mut id = RULESET_COUNT.may_load(deps.storage)?.unwrap_or_default();
    for ruleset in to_add {
        rulesets().save(deps.storage, id.u128(), &ruleset)?;
        id = id.checked_add(Uint128::one())?;
    }
    RULESET_COUNT.save(deps.storage, &id)?;

    Ok(Response::new()
        .add_attribute("action", "update_rulesets")
        .add_attribute("ruleset_count", Uint128::from(id)))
}

pub fn jail_competition(
    deps: DepsMut,
    sender: Addr,
    id: Uint128,
) -> Result<Response, ContractError> {
    //assert the sender is a competition module
    if !competition_modules().has(deps.storage, sender.clone()) {
        return Err(ContractError::Unauthorized {});
    }

    let dao = PrePropose::default().dao.load(deps.storage)?;
    let proposal_module = PrePropose::default().proposal_module.load(deps.storage)?;
    let voting_module: Addr = deps
        .querier
        .query_wasm_smart(dao, &dao_interface::msg::QueryMsg::VotingModule {})?;
    let cw4_group: Addr = deps.querier.query_wasm_smart(
        voting_module,
        &dao_voting_cw4::msg::QueryMsg::GroupContract {},
    )?;

    Ok(Response::new()
        .add_attribute("action", "jail_competition")
        .add_attribute("sender", sender.to_string())
        .add_attribute("id", id)
        .add_message(create_competition_proposals(
            deps.as_ref(),
            Uint128::from(id),
            &sender,
            &cw4_group,
            &proposal_module,
        )?))
}
