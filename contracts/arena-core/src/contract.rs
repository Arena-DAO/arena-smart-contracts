use crate::{
    execute::{self, COMPETITION_MODULE_REPLY_ID},
    msg::{ExecuteExt, ExecuteMsg, InstantiateExt, InstantiateMsg, PrePropose, QueryExt, QueryMsg},
    query,
    state::{competition_modules, CompetitionModule, COMPETITION_MODULES_COUNT, TAX},
    ContractError,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;
use dao_core::msg::ExecuteMsg as DAOCoreExecuteMsg;
use dao_interface::ModuleInstantiateCallback;

pub(crate) const CONTRACT_NAME: &str = "crates.io:arena-dao-core";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const ITEM_KEY: &str = "Arena";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let resp =
        PrePropose::default().instantiate(deps.branch(), env.clone(), info.clone(), msg.clone())?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(instantiate_extension(
        resp,
        deps.branch(),
        info,
        env,
        msg.extension,
    )?)
}

pub fn instantiate_extension(
    prepropose_response: Response,
    mut deps: DepsMut,
    info: MessageInfo,
    env: Env,
    extension: InstantiateExt,
) -> Result<Response, ContractError> {
    let dao = PrePropose::default().dao.load(deps.storage)?;
    crate::execute::update_tax(deps.branch(), &env, extension.tax)?;
    crate::execute::update_rulesets(deps.branch(), extension.rulesets, vec![])?;
    let competition_response = crate::execute::update_competition_modules(
        deps.branch(),
        &env,
        info,
        extension.competition_modules_instantiate_info,
        vec![],
    )?;
    TAX.save(deps.storage, &extension.tax, env.block.height)?;
    Ok(prepropose_response
        .add_submessages(competition_response.messages)
        .set_data(to_binary(&ModuleInstantiateCallback {
            msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: dao.to_string(),
                msg: to_binary(&DAOCoreExecuteMsg::SetItem {
                    key: ITEM_KEY.to_string(),
                    value: env.contract.address.to_string(),
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
        ExecuteMsg::Propose { msg: _ } => return Err(ContractError::Unauthorized {}),
        ExecuteMsg::Extension { msg } => match msg {
            ExecuteExt::UpdateCompetitionModules { to_add, to_disable } => {
                execute::update_competition_modules(deps, &env, info, to_add, to_disable)
            }
            ExecuteExt::UpdateRulesets { to_add, to_disable } => {
                execute::update_rulesets(deps, to_add, to_disable)
            }
            ExecuteExt::UpdateTax { tax } => execute::update_tax(deps, &env, tax),
        },
        // Default pre-propose-base behavior for all other messages
        _ => Ok(PrePropose::default().execute(deps, env, info, msg)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        COMPETITION_MODULE_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg)?;
            let module_addr = deps.api.addr_validate(&res.contract_address)?;

            let competition_module = CompetitionModule {
                addr: module_addr.clone(),
                is_enabled: true,
            };

            let competition_modules_count = COMPETITION_MODULES_COUNT.load(deps.storage)?;
            competition_modules().save(
                deps.storage,
                competition_modules_count.u128(),
                &competition_module,
            )?;
            COMPETITION_MODULES_COUNT.save(
                deps.storage,
                &competition_modules_count.checked_add(Uint128::one())?,
            )?;

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
        /*DAO_REPLY_ID => {
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
                    &env.contract.address,
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
        }*/
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryExtension { msg } => match msg {
            QueryExt::CompetitionModules {
                start_after,
                limit,
                include_disabled,
            } => to_binary(&query::competition_modules(
                deps,
                start_after,
                limit,
                include_disabled,
            )?),
            QueryExt::DumpState {} => to_binary(&query::dump_state(deps)?),
            QueryExt::Rulesets {
                skip,
                limit,
                include_disabled,
            } => to_binary(&query::rulesets(deps, skip, limit, include_disabled)?),
            QueryExt::Tax { height } => to_binary(&query::tax(deps, env, height)?),
        },
        _ => PrePropose::default().query(deps, env, msg),
    }
}
