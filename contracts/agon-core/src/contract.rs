#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, SubMsg,
    Uint128,
};
use cw2::set_contract_version;
use cw_utils::{parse_reply_instantiate_data, ParseReplyError};

use crate::{
    execute::{
        self, create_wager_proposals, COMPETITION_MODULE_REPLY_ID, COMPETITION_REPLY_ID,
        DAO_REPLY_ID, ESCROW_REPLY_ID,
    },
    models::{ModuleInstantiateInfo, WagerStatus},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    query,
    state::{DAO, TEMP_WAGER, WAGERS},
    ContractError,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:agon-core";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(instantiate_contract(
        deps,
        env,
        info.sender,
        msg.competition_modules_instantiate_info,
    )?)
}

pub fn instantiate_contract(
    deps: DepsMut,
    env: Env,
    dao: Addr,
    competition_modules_instantiate_info: Vec<ModuleInstantiateInfo>,
) -> StdResult<Response> {
    DAO.save(deps.storage, &dao)?;
    let competition_module_msgs: Vec<SubMsg> = competition_modules_instantiate_info
        .into_iter()
        .map(|info| info.into_wasm_msg(Some(env.contract.address.to_string())))
        .map(|wasm| SubMsg::reply_on_success(wasm, COMPETITION_MODULE_REPLY_ID))
        .collect();
    Ok(Response::new().add_submessages(competition_module_msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateCompetitionModules { to_add, to_disable } => {
            execute::update_competition_modules(deps, env, info, to_add, to_disable)
        }
        ExecuteMsg::CreateWager {
            wager_dao,
            expiration,
            wager_amount,
            stake,
            rules,
            ruleset,
            escrow_code_id,
        } => execute::create_wager(
            deps,
            env,
            wager_dao,
            expiration,
            escrow_code_id,
            wager_amount,
            stake,
            rules,
            ruleset,
        ),
        ExecuteMsg::HandleWager { id, distribution } => {
            execute::handle_wager(deps, info, id, distribution)
        }
        ExecuteMsg::UpdateRulesets { to_add, to_disable } => {
            execute::update_rulesets(deps, to_add, to_disable)
        }
        ExecuteMsg::JailWager { id } => execute::jail_wager(deps, env, id),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        COMPETITION_MODULE_REPLY_ID => {
            let result = msg
                .result
                .into_result()
                .map_err(|x| ContractError::ParseReply(ParseReplyError::SubMsgFailure(x)))?;
            todo!()

            /*Ok(Response::new()
            .add_attribute("action", "reply_competition_module")
            .add_attribute("competition_module", competition_module.addr)
            .add_attribute("competition_module_name", competition_module.name))*/
        }
        DAO_REPLY_ID => {
            let result = parse_reply_instantiate_data(msg)?;
            let addr = deps.api.addr_validate(&result.contract_address)?;

            let wager_id = TEMP_WAGER.may_load(deps.storage)?;
            if wager_id.is_none() {
                return Err(ContractError::UnknownWagerId { id: 0u128 });
            }
            let wager_id = wager_id.unwrap();

            WAGERS.update(deps.storage, wager_id, |x| -> Result<_, ContractError> {
                match x {
                    Some(mut wager) => {
                        if wager.dao != env.contract.address {
                            return Err(ContractError::Unauthorized {});
                        }
                        wager.dao = addr.clone();
                        Ok(wager)
                    }
                    None => Err(ContractError::UnknownWagerId { id: wager_id }),
                }
            })?;

            Ok(Response::new()
                .add_attribute("action", "reply_dao")
                .add_attribute("wager", Uint128::from(wager_id))
                .add_attribute("dao_addr", addr.clone())
                .add_message(create_wager_proposals(
                    deps.as_ref(),
                    env,
                    &addr,
                    Uint128::from(wager_id),
                )?))
        }
        ESCROW_REPLY_ID => {
            let result = parse_reply_instantiate_data(msg)?;
            let addr = deps.api.addr_validate(&result.contract_address)?;
            let wager_id = TEMP_WAGER.load(deps.storage)?;

            WAGERS.update(deps.storage, wager_id, |x| -> Result<_, ContractError> {
                match x {
                    Some(mut wager) => {
                        if wager.escrow != env.contract.address {
                            return Err(ContractError::Unauthorized {});
                        }
                        wager.escrow = addr.clone();
                        Ok(wager)
                    }
                    None => Err(ContractError::UnknownWagerId { id: wager_id }),
                }
            })?;

            Ok(Response::new()
                .add_attribute("action", "reply_escrow")
                .add_attribute("wager", Uint128::from(wager_id))
                .add_attribute("escrow_addr", addr))
        }
        COMPETITION_REPLY_ID => {
            parse_reply_instantiate_data(msg)?;
            let wager_id = TEMP_WAGER.load(deps.storage)?;

            WAGERS.update(deps.storage, wager_id, |x| -> Result<_, ContractError> {
                match x {
                    Some(mut wager) => {
                        wager.status = WagerStatus::Inactive;
                        Ok(wager)
                    }
                    None => Err(ContractError::UnknownWagerId { id: wager_id }),
                }
            })?;

            Ok(Response::new()
                .add_attribute("action", "reply_result")
                .add_attribute("wager", Uint128::from(wager_id)))
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::CompetitionModules { start_after, limit } => {
            to_binary(&query::competition_modules(deps, start_after, limit)?)
        }
        QueryMsg::DumpState {} => to_binary(&query::dump_state(deps)?),
        QueryMsg::Rulesets { start_after, limit } => {
            todo!()
        }
        QueryMsg::DAO {} => todo!(),
        QueryMsg::Tax { height } => todo!(),
    }
}
