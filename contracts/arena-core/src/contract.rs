use crate::{
    execute::{self, COMPETITION_MODULE_REPLY_ID},
    migrate, query,
    state::{
        competition_modules, rulesets, CompetitionModule, ARENA_TAX_CONFIG,
        COMPETITION_CATEGORIES_COUNT, KEYS, PAYMENT_REGISTRY, RATING_PERIOD, RULESETS_COUNT,
    },
    ContractError,
};
use arena_interface::core::{
    ExecuteExt, ExecuteMsg, InstantiateExt, InstantiateMsg, MigrateExt, MigrateMsg, PrePropose,
    QueryExt, QueryMsg,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError, Uint128, WasmMsg,
};
use cw2::{ensure_from_older_version, set_contract_version};
use cw_utils::parse_reply_instantiate_data;
use dao_interface::{msg::ExecuteMsg as DAOCoreExecuteMsg, state::ModuleInstantiateCallback};

pub(crate) const CONTRACT_NAME: &str = "crates.io:arena-core";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const ITEM_KEY: &str = "Arena";
pub const WAGERS_KEY: &str = "Wagers";
pub const LEAGUES_KEY: &str = "Leagues";
pub const TOURNAMENTS_KEY: &str = "Tournaments";

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
    deps: DepsMut,
    env: Env,
    extension: InstantiateExt,
) -> Result<Response, ContractError> {
    let dao = PrePropose::default().dao.load(deps.storage)?;
    RULESETS_COUNT.save(deps.storage, &Uint128::zero())?;
    COMPETITION_CATEGORIES_COUNT.save(deps.storage, &Uint128::zero())?;
    let mut msgs = vec![];

    // Update Tax
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_json_binary(&ExecuteMsg::Extension {
            msg: ExecuteExt::UpdateTax { tax: extension.tax },
        })?,
        funds: vec![],
    }));

    // Update Categories
    if let Some(categories) = extension.categories {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateCategories {
                    to_add: Some(categories),
                    to_edit: None,
                },
            })?,
            funds: vec![],
        }));
    }

    // Update Rulesets
    if let Some(rulesets) = extension.rulesets {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateRulesets {
                    to_add: Some(rulesets),
                    to_disable: None,
                },
            })?,
            funds: vec![],
        }));
    }

    // Update Rating Period
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_json_binary(&ExecuteMsg::Extension {
            msg: ExecuteExt::UpdateRatingPeriod {
                period: extension.rating_period,
            },
        })?,
        funds: vec![],
    }));

    // Save Arena Tax Config
    ARENA_TAX_CONFIG.save(deps.storage, &extension.tax_configuration)?;

    // Update Competition Modules
    if let Some(competition_modules_instantiate_info) =
        extension.competition_modules_instantiate_info
    {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::UpdateCompetitionModules {
                    to_add: Some(competition_modules_instantiate_info),
                    to_disable: None,
                },
            })?,
            funds: vec![],
        }));
    }

    // Set Payment Registry
    if let Some(payment_registry) = extension.payment_registry {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::SetPaymentRegistry {
                    addr: payment_registry,
                },
            })?,
            funds: vec![],
        }));
    }

    Ok(prepropose_response
        .add_messages(msgs)
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
        ExecuteMsg::Extension { msg } => {
            // Check authorization for all Extension messages except AdjustRatings
            if !matches!(msg, ExecuteExt::AdjustRatings { .. })
                && env.contract.address != info.sender
            {
                let dao = PrePropose::default().dao.load(deps.storage)?;
                if dao != info.sender {
                    return Err(ContractError::Unauthorized {});
                }
            }

            match msg {
                ExecuteExt::UpdateCompetitionModules { to_add, to_disable } => {
                    execute::update_competition_modules(deps, info.sender, to_add, to_disable)
                }
                ExecuteExt::UpdateRulesets { to_add, to_disable } => {
                    execute::update_rulesets(deps, to_add, to_disable)
                }
                ExecuteExt::UpdateTax { tax } => execute::update_tax(deps, &env, tax),
                ExecuteExt::UpdateCategories { to_add, to_edit } => {
                    execute::update_categories(deps, to_add, to_edit)
                }
                ExecuteExt::AdjustRatings {
                    category_id,
                    member_results,
                } => execute::adjust_ratings(deps, env, info, category_id, member_results),
                ExecuteExt::UpdateRatingPeriod { period } => {
                    execute::update_rating_period(deps, period)
                }
                ExecuteExt::UpdateEnrollmentModules { to_add, to_remove } => {
                    execute::update_enrollment_modules(deps, to_add, to_remove)
                }
                ExecuteExt::SetPaymentRegistry { addr } => {
                    execute::set_payment_registry(deps, addr)
                }
            }
        }
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

            competition_modules().save(deps.storage, &module_addr, &competition_module)?;
            KEYS.save(deps.storage, key.clone(), &module_addr, env.block.height)?;

            // Check for module instantiation callbacks
            let callback_msgs = match res.data {
                Some(data) => from_json::<ModuleInstantiateCallback>(&data)
                    .map(|m| m.msgs)
                    .unwrap_or_else(|_| vec![]),
                None => vec![],
            };

            Ok(Response::default()
                .add_attribute("action", "reply_competition_module")
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
            QueryExt::TaxConfig { height } => {
                to_json_binary(&query::arena_fee_config(deps, height)?)
            }
            QueryExt::Rating { category_id, addr } => {
                to_json_binary(&query::rating(deps, category_id, addr)?)
            }
            QueryExt::RatingLeaderboard {
                category_id,
                start_after,
                limit,
            } => to_json_binary(&query::rating_leaderboard(
                deps,
                category_id,
                start_after,
                limit,
            )?),
            QueryExt::IsValidEnrollmentModule { addr } => {
                to_json_binary(&query::is_valid_enrollment_module(deps, addr)?)
            }
            QueryExt::RatingPeriod {} => to_json_binary(&RATING_PERIOD.may_load(deps.storage)?),
            QueryExt::PaymentRegistry {} => {
                to_json_binary(&PAYMENT_REGISTRY.may_load(deps.storage)?)
            }
        },
        _ => PrePropose::default().query(deps, env, msg),
    };

    Ok(binary_result?)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(mut deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    match msg {
        MigrateMsg::Extension { msg } => match msg {
            MigrateExt::FromCompatible {} => {
                if version.major == 1 && version.minor <= 4 {
                    migrate::from_v1_3_to_v1_4(deps.branch())?;
                }

                if version.major == 1 && version.minor < 6 {
                    migrate::from_v1_4_to_v1_6(deps.branch())?;
                }

                if version.major == 1 && version.minor < 8 {
                    // Rulesets state has changed. There's nothing important there atm, so we can just clear the state.
                    rulesets().clear(deps.storage);
                }
            }
            MigrateExt::Patch(patch) => {
                if patch.as_str() == "v1.4" {
                    migrate::from_v1_3_to_v1_4(deps.branch())?;
                }
            }
        },
        MigrateMsg::FromUnderV250 { policy: _ } => {
            // Need to set_contract_version before using the base migrate because of versioning checks
            set_contract_version(deps.storage, CONTRACT_NAME, "2.4.1")?;
            PrePropose::default().migrate(deps.branch(), msg)?;
        }
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
