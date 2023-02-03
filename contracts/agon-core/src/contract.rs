#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;
use dao_core::msg::ExecuteMsg as DAOCoreExecuteMsg;
use dao_interface::{ModuleInstantiateCallback, ModuleInstantiateInfo};

use crate::{
    execute::{
        self, create_wager_proposals, COMPETITION_MODULE_REPLY_ID, COMPETITION_REPLY_ID,
        DAO_REPLY_ID, ESCROW_REPLY_ID,
    },
    models::{CompetitionModule, WagerStatus},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    query,
    state::{
        COMPETITION_MODULES, COMPETITION_MODULES_COUNT, DAO, PROPOSAL_MODULE, TEMP_WAGER, WAGERS,
    },
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
    let dao = deps.api.addr_validate(&msg.dao)?;
    Ok(instantiate_contract(
        deps,
        &env.contract.address,
        &dao,
        &info.sender,
        msg.competition_modules_instantiate_info,
    )?)
}

pub fn instantiate_contract(
    deps: DepsMut,
    contract: &Addr,
    dao: &Addr,
    proposal_module: &Addr,
    competition_modules_instantiate_info: Vec<ModuleInstantiateInfo>,
) -> StdResult<Response> {
    DAO.save(deps.storage, &dao)?;
    let msgs: Vec<SubMsg> = competition_modules_instantiate_info
        .into_iter()
        .map(|info| info.into_wasm_msg(contract.clone()))
        .map(|wasm| SubMsg::reply_on_success(wasm, COMPETITION_MODULE_REPLY_ID))
        .collect();
    PROPOSAL_MODULE.save(deps.storage, proposal_module)?;
    Ok(Response::new()
        .add_submessages(msgs)
        .set_data(to_binary(&ModuleInstantiateCallback {
            msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: dao.to_string(),
                msg: to_binary(&DAOCoreExecuteMsg::SetItem {
                    key: "Agon".to_string(),
                    value: contract.to_string(),
                })?,
                funds: vec![],
            })],
        })?))
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
            let res = parse_reply_instantiate_data(msg)?;
            let module_addr = deps.api.addr_validate(&res.contract_address)?;

            let competition_module = CompetitionModule {
                addr: module_addr.clone(),
            };

            COMPETITION_MODULES.save(deps.storage, module_addr, &competition_module)?;
            COMPETITION_MODULES_COUNT.update::<_, StdError>(deps.storage, |count| {
                Ok(count.checked_add(Uint128::one())?)
            })?;

            // Check for module instantiation callbacks
            let callback_msgs = match res.data {
                Some(data) => from_binary::<ModuleInstantiateCallback>(&data)
                    .map(|m| m.msgs)
                    .unwrap_or_else(|_| vec![]),
                None => vec![],
            };

            Ok(Response::default()
                .add_attribute("competition_module".to_string(), res.contract_address)
                .add_messages(callback_msgs))
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
