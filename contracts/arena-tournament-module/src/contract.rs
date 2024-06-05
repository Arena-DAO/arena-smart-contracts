#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response, StdResult,
};
use cw2::{ensure_from_older_version, set_contract_version};
use cw_competition::msg::{ExecuteBase, QueryBase};
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
pub type CompetitionModule =
    CompetitionModuleContract<Empty, ExecuteExt, QueryExt, TournamentExt, TournamentInstantiateExt>;

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
            category_id,
            host,
            escrow,
            name,
            description,
            expiration,
            rules,
            rulesets,
            instantiate_extension,
        } => {
            let response = CompetitionModule::default().execute_create_competition(
                &mut deps,
                &env,
                category_id,
                host,
                escrow,
                name,
                description,
                expiration,
                rules,
                rulesets,
                &instantiate_extension,
            )?;

            execute::instantiate_tournament(
                deps,
                response,
                instantiate_extension.teams,
                instantiate_extension.elimination_type,
            )
        }
        ExecuteBase::Extension { msg } => match msg {
            ExecuteExt::ProcessMatch {
                tournament_id,
                match_results,
            } => execute::process_matches(deps, info, tournament_id, match_results),
        },
        ExecuteBase::ProcessCompetition {
            competition_id: _,
            distribution: _,
        } => Err(ContractError::InvalidExecute),
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
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let _version = ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
