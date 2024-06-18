use arena_interface::competition::msg::{ExecuteBase, QueryBase};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response, StdResult,
};
use cw2::{ensure_from_older_version, set_contract_version};
use cw_competition_base::{contract::CompetitionModuleContract, error::CompetitionError};

use crate::{
    execute, migrate,
    msg::{
        ExecuteExt, ExecuteMsg, InstantiateMsg, LeagueInstantiateExt, LeagueQueryExt, MigrateMsg,
        QueryMsg,
    },
    query,
    state::LeagueExt,
    ContractError,
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:arena-league-module";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub type CompetitionModule<'a> = CompetitionModuleContract<
    'a,
    Empty,
    ExecuteExt,
    LeagueQueryExt,
    LeagueExt,
    LeagueInstantiateExt,
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
            banner,
            should_activate_on_funded,
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
                banner,
                should_activate_on_funded,
                &instantiate_extension,
            )?;

            execute::instantiate_rounds(deps, response, instantiate_extension.teams)
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
            ExecuteExt::AddPointAdjustments {
                league_id,
                addr,
                point_adjustments,
            } => execute::add_point_adjustments(deps, info, league_id, addr, point_adjustments),
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
            LeagueQueryExt::Leaderboard { league_id, round } => {
                to_json_binary(&query::leaderboard(deps, league_id, round)?)
            }
            LeagueQueryExt::Round {
                league_id,
                round_number,
            } => to_json_binary(&query::round(deps, league_id, round_number)?),
            LeagueQueryExt::PointAdjustments {
                league_id,
                start_after,
                limit,
            } => to_json_binary(&query::point_adjustments(
                deps,
                league_id,
                start_after,
                limit,
            )?),
            LeagueQueryExt::DumpState {
                league_id,
                round_number,
            } => to_json_binary(&query::dump_state(deps, league_id, round_number)?),
        },
        _ => CompetitionModule::default().query(deps, env, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(mut deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let competition_module = CompetitionModule::default();
    let version = ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if version.major == 1 && version.minor == 3 {
        migrate::from_v1_3_to_v_1_4(deps.branch())?;
    }
    if version.major == 1 && version.minor < 7 {
        competition_module.migrate_from_v1_6_to_v1_7(deps.branch())?;
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
