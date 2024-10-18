use arena_interface::competition::msg::{ExecuteBase, QueryBase};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure_eq, to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply,
    Response, StdResult, WasmMsg,
};
use cw2::{ensure_from_older_version, set_contract_version};
use cw_competition_base::{contract::CompetitionModuleContract, error::CompetitionError};

use crate::{
    execute,
    msg::{
        ExecuteExt, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryExt, QueryMsg,
        TournamentInstantiateExt,
    },
    query,
    state::TournamentExt,
    ContractError,
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:arena-tournament-module";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub type CompetitionModule<'a> = CompetitionModuleContract<
    'a,
    Empty,
    ExecuteExt,
    QueryExt,
    TournamentExt,
    TournamentInstantiateExt,
>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
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
) -> Result<Response, ContractError> {
    match msg {
        ExecuteBase::CreateCompetition {
            host,
            category_id,
            escrow,
            name,
            description,
            expiration,
            rules,
            rulesets,
            banner,
            instantiate_extension,
            group_contract,
        } => Ok(CompetitionModule::default()
            .execute_create_competition(
                &mut deps,
                &env,
                &info,
                host,
                category_id,
                escrow,
                name,
                description,
                expiration,
                rules,
                rulesets,
                banner,
                group_contract,
                instantiate_extension,
            )?
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_json_binary(&ExecuteMsg::Extension {
                    msg: ExecuteExt::InstantiateTournament {},
                })?,
                funds: vec![],
            }))),
        ExecuteBase::Extension { msg } => match msg {
            ExecuteExt::ProcessMatch {
                tournament_id,
                match_results,
            } => execute::process_matches(deps, info, tournament_id, match_results),
            ExecuteExt::InstantiateTournament {} => {
                execute::instantiate_tournament(deps, env, info)
            }
        },
        ExecuteBase::ProcessCompetition {
            competition_id,
            distribution,
        } => {
            let competition = CompetitionModule::default()
                .competitions
                .load(deps.storage, competition_id.u128())?;
            ensure_eq!(
                info.sender.clone(),
                competition.admin_dao,
                ContractError::CompetitionError(CompetitionError::Unauthorized {})
            );

            Ok(CompetitionModule::default().execute_process_competition(
                deps,
                info,
                competition_id,
                distribution,
                None,
            )?)
        }
        _ => Ok(CompetitionModule::default().execute(deps, env, info, msg)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, CompetitionError> {
    CompetitionModule::default().reply(deps, env, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryBase::QueryExtension { msg } => match msg {
            QueryExt::Bracket {
                tournament_id,
                start_after,
            } => to_json_binary(&query::query_bracket(deps, tournament_id, start_after)?),
            QueryExt::Match {
                tournament_id,
                match_number,
            } => to_json_binary(&query::query_match(deps, tournament_id, match_number)?),
        },
        _ => CompetitionModule::default().query(deps, env, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(mut deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let competition_module = CompetitionModule::default();
    let version = ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if version.major == 1 && version.minor < 7 {
        competition_module.migrate_from_v1_6_to_v1_7(deps.branch())?;
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
