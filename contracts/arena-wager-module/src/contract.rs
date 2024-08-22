use std::collections::HashSet;

use arena_interface::ratings::MemberResult;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, Decimal, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response, StdResult, SubMsg,
};
use cw2::{ensure_from_older_version, set_contract_version};
use cw_competition_base::{contract::CompetitionModuleContract, error::CompetitionError};

use crate::msg::{
    ExecuteExt, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryExt, QueryMsg, Wager, WagerExt,
    WagerInstantiateExt,
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:arena-wager-module";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub type CompetitionModule<'a> =
    CompetitionModuleContract<'a, Empty, ExecuteExt, QueryExt, WagerExt, WagerInstantiateExt>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, CompetitionError> {
    let resp = CompetitionModule::default().instantiate(deps.branch(), env, info, msg)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, CompetitionError> {
    match msg {
        ExecuteMsg::ProcessCompetition {
            competition_id,
            distribution,
        } => CompetitionModule::default().execute_process_competition(
            deps.branch(),
            info,
            competition_id,
            distribution,
            Some(post_processing),
        ),
        _ => CompetitionModule::default().execute(deps, env, info, msg),
    }
}

fn post_processing(deps: DepsMut, competition: &Wager) -> Result<Option<SubMsg>, CompetitionError> {
    if !CompetitionModule::default().query_is_dao_member(
        deps.as_ref(),
        &competition.host,
        competition.start_height,
    ) {
        return Ok(None);
    }

    if let Some(category_id) = competition.category_id {
        if let Some(registered_members) = &competition.extension.registered_members {
            // This will be in state
            let result = CompetitionModule::default()
                .competition_result
                .load(deps.storage, competition.id.u128())?;

            return Ok(match result {
                Some(result) => {
                    let registered_members_set: HashSet<_> = registered_members.iter().collect();

                    // If all items of the distribution are part of the registered members list, then we can execute a valid rating update
                    if result
                        .member_percentages
                        .iter()
                        .all(|x| registered_members_set.contains(&x.addr))
                    {
                        let member_result1 = MemberResult {
                            addr: result.member_percentages[0].addr.clone(),
                            result: result.member_percentages[0].percentage,
                        };

                        let member_result2 = if result.member_percentages.len() > 1 {
                            MemberResult {
                                addr: result.member_percentages[1].addr.clone(),
                                result: result.member_percentages[1].percentage,
                            }
                        } else {
                            MemberResult {
                                addr: registered_members
                                    .iter()
                                    .find(|addr| *addr != member_result1.addr)
                                    .unwrap()
                                    .clone(),
                                result: Decimal::zero(),
                            }
                        };

                        Some(CompetitionModule::default().trigger_rating_adjustment(
                            deps.storage,
                            category_id,
                            vec![(member_result1, member_result2)],
                        )?)
                    } else {
                        None
                    }
                }
                // This is a Draw
                None => Some(CompetitionModule::default().trigger_rating_adjustment(
                    deps.storage,
                    category_id,
                    vec![(
                        MemberResult {
                            addr: registered_members[0].clone(),
                            result: Decimal::percent(50),
                        },
                        MemberResult {
                            addr: registered_members[1].clone(),
                            result: Decimal::percent(50),
                        },
                    )],
                )?),
            });
        }
    }

    Ok(None)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, CompetitionError> {
    CompetitionModule::default().reply(deps, env, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    CompetitionModule::default().query(deps, env, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(
    mut deps: DepsMut,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, CompetitionError> {
    let competition_module = CompetitionModule::default();
    let version = ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if version.major == 1 && version.minor < 7 {
        competition_module.migrate_from_v1_6_to_v1_7(deps.branch())?;
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
