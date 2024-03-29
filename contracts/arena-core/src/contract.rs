use crate::{
    execute::{self, COMPETITION_MODULE_REPLY_ID},
    query,
    state::{
        competition_modules, CompetitionModule, COMPETITION_CATEGORIES_COUNT,
        COMPETITION_MODULES_COUNT, KEYS, RULESETS_COUNT,
    },
    ContractError,
};
use arena_core_interface::msg::{
    ExecuteExt, ExecuteMsg, InstantiateExt, InstantiateMsg, MigrateMsg, PrePropose, QueryExt,
    QueryMsg,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;
use dao_interface::{msg::ExecuteMsg as DAOCoreExecuteMsg, state::ModuleInstantiateCallback};

pub(crate) const CONTRACT_NAME: &str = "crates.io:arena-core";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const ITEM_KEY: &str = "Arena";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let resp = PrePropose::default().instantiate(deps.branch(), env.clone(), info, msg.clone())?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    instantiate_extension(resp, deps.branch(), env, msg.extension)
}

pub fn instantiate_extension(
    prepropose_response: Response,
    mut deps: DepsMut,
    env: Env,
    extension: InstantiateExt,
) -> Result<Response, ContractError> {
    let dao = PrePropose::default().dao.load(deps.storage)?;
    COMPETITION_MODULES_COUNT.save(deps.storage, &Uint128::zero())?;
    RULESETS_COUNT.save(deps.storage, &Uint128::zero())?;
    COMPETITION_CATEGORIES_COUNT.save(deps.storage, &Uint128::zero())?;
    crate::execute::update_tax(deps.branch(), &env, dao.clone(), extension.tax)?;
    crate::execute::update_categories(deps.branch(), dao.clone(), extension.categories, vec![])?;
    crate::execute::update_rulesets(deps.branch(), dao.clone(), extension.rulesets, vec![])?;
    let competition_response = crate::execute::update_competition_modules(
        deps.branch(),
        dao.clone(),
        extension.competition_modules_instantiate_info,
        vec![],
    )?;
    Ok(prepropose_response
        .add_submessages(competition_response.messages)
        .set_data(to_json_binary(&ModuleInstantiateCallback {
            msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: dao.to_string(),
                msg: to_json_binary(&DAOCoreExecuteMsg::SetItem {
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
        ExecuteMsg::Propose { msg } => Ok(execute::propose(deps, env, info, msg)?),
        ExecuteMsg::Extension { msg } => match msg {
            ExecuteExt::UpdateCompetitionModules { to_add, to_disable } => {
                execute::update_competition_modules(deps, info.sender, to_add, to_disable)
            }
            ExecuteExt::UpdateRulesets { to_add, to_disable } => {
                execute::update_rulesets(deps, info.sender, to_add, to_disable)
            }
            ExecuteExt::UpdateTax { tax } => execute::update_tax(deps, &env, info.sender, tax),
            ExecuteExt::UpdateCategories { to_add, to_edit } => {
                execute::update_categories(deps, info.sender, to_add, to_edit)
            }
        },
        // Default pre-propose-base behavior for all other messages
        _ => Ok(PrePropose::default().execute(deps, env, info, msg)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        COMPETITION_MODULE_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg.clone())?;
            let module_addr = deps.api.addr_validate(&res.contract_address)?;
            let key = msg
                .result
                .unwrap() //this result is handled in parse_reply
                .events
                .iter()
                .find_map(|e| {
                    e.attributes.iter().find_map(|attr| {
                        if attr.key == "key" {
                            Some(attr.value.clone())
                        } else {
                            None
                        }
                    })
                })
                .ok_or_else(|| {
                    StdError::generic_err("Unable to find the module key.".to_string())
                })?;

            let competition_module = CompetitionModule {
                addr: module_addr.clone(),
                is_enabled: true,
                key: key.clone(),
            };

            competition_modules().save(deps.storage, module_addr.clone(), &competition_module)?;
            KEYS.save(deps.storage, key.clone(), &module_addr, env.block.height)?;
            COMPETITION_MODULES_COUNT.update(deps.storage, |x| -> StdResult<_> {
                Ok(x.checked_add(Uint128::one())?)
            })?;

            // Check for module instantiation callbacks
            let callback_msgs = match res.data {
                Some(data) => from_json::<ModuleInstantiateCallback>(&data)
                    .map(|m| m.msgs)
                    .unwrap_or_else(|_| vec![]),
                None => vec![],
            };

            Ok(Response::default()
                .add_attribute("key", key)
                .add_attribute("competition_module".to_string(), res.contract_address)
                .add_messages(callback_msgs))
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let binary_result = match msg {
        QueryMsg::QueryExtension { msg } => match msg {
            QueryExt::CompetitionModules {
                start_after,
                limit,
                include_disabled,
            } => to_json_binary(&query::competition_modules(
                deps,
                start_after,
                limit,
                include_disabled,
            )?),
            QueryExt::Rulesets {
                category_id,
                start_after,
                limit,
                include_disabled,
            } => to_json_binary(&query::rulesets(
                deps,
                category_id,
                start_after,
                limit,
                include_disabled,
            )?),
            QueryExt::Ruleset { id } => to_json_binary(&query::ruleset(deps, id)?),
            QueryExt::Categories {
                start_after,
                limit,
                include_disabled,
            } => to_json_binary(&query::categories(
                deps,
                start_after,
                limit,
                include_disabled,
            )?),
            QueryExt::Category { id } => to_json_binary(&query::category(deps, id)?),
            QueryExt::Tax { height } => to_json_binary(&query::tax(deps, env, height)?),
            QueryExt::CompetitionModule { query } => {
                to_json_binary(&query::competition_module(deps, env, query)?)
            }
            QueryExt::DumpState {} => to_json_binary(&query::dump_state(deps, env)?),
            QueryExt::IsValidCategoryAndRulesets {
                category_id,
                rulesets,
            } => to_json_binary(&query::is_valid_category_and_rulesets(
                deps,
                category_id,
                rulesets,
            )),
        },
        _ => PrePropose::default().query(deps, env, msg),
    };

    Ok(binary_result?)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
