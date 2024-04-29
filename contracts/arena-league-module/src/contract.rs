#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
};
use cw2::set_contract_version;
use cw_competition::msg::{ExecuteBase, QueryBase};
use cw_competition_base::{contract::CompetitionModuleContract, error::CompetitionError};

use crate::{
    execute,
    msg::{
        CompetitionExt, CompetitionInstantiateExt, ExecuteExt, ExecuteMsg, InstantiateMsg,
        MigrateMsg, QueryExt, QueryMsg,
    },
    query,
    state::TournamentExt,
    ContractError,
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:arena-league-module";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub type CompetitionModule = CompetitionModuleContract<
    TournamentExt,
    ExecuteExt,
    QueryExt,
    CompetitionExt,
    CompetitionInstantiateExt,
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

            execute::instantiate_rounds(
                deps,
                response,
                instantiate_extension.teams,
                instantiate_extension.distribution,
            )
        }
        ExecuteBase::Extension { msg } => match msg {
            ExecuteExt::ProcessMatch {
                league_id,
                round_number,
                match_results,
            } => execute::process_matches(deps, info, league_id, round_number, match_results),
            ExecuteExt::UpdateDistribution {
                league_id,
                distribution,
            } => execute::update_distribution(deps, info, league_id, distribution),
        },
        ExecuteBase::ProcessCompetition {
            competition_id: _,
            distribution: _,
            tax_cw20_msg: _,
            tax_cw721_msg: _,
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
            QueryExt::Leaderboard { league_id, round } => {
                to_json_binary(&query::leaderboard(deps, league_id, round)?)
            }
            QueryExt::Round {
                league_id,
                round_number,
            } => to_json_binary(&query::round(deps, league_id, round_number)?),
        },
        _ => CompetitionModule::default().query(deps, env, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, CompetitionError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
