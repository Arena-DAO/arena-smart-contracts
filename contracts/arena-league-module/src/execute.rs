use cosmwasm_std::{
    Addr, Deps, DepsMut, Env, MessageInfo, OverflowError, OverflowOperation, Response, StdError,
    StdResult, Uint128, Uint64,
};
use cw_balance::MemberShare;
use cw_utils::Duration;
use itertools::Itertools;
use std::ops::Add;

use crate::{
    contract::CompetitionModule,
    msg::MatchResult,
    state::{Match, Round, DISTRIBUTION, MATCHES, ROUNDS},
    ContractError,
};

#[allow(clippy::too_many_arguments)]
pub fn instantiate_rounds(
    deps: DepsMut,
    env: Env,
    response: Response,
    teams: Vec<String>,
    round_duration: Duration,
) -> Result<Response, ContractError> {
    if teams.iter().unique().count() != teams.len() {
        return Err(ContractError::StdError(StdError::GenericErr {
            msg: "Teams cannot have duplicates".to_string(),
        }));
    }

    // Convert team names to addresses
    let team_addresses: Vec<Addr> = teams
        .iter()
        .map(|name| deps.api.addr_validate(name))
        .collect::<StdResult<_>>()?;
    let team_count = team_addresses.len();

    // Determine the number of rounds and matches per round
    let rounds = if team_count % 2 == 1 {
        team_count
    } else {
        team_count - 1
    };
    let matches_per_round = (rounds + 1) / 2;

    // Table of teams, starting from 1 to n
    let mut table: Vec<usize> = (1..=(rounds + 1)).collect();

    // Stores the rounds with the corresponding matches
    let mut matches: Vec<Vec<(usize, usize)>> = Vec::new();
    for r in 0..rounds {
        matches.push(vec![]);
        for m in 0..matches_per_round {
            // Ignore the dummy team
            if table[table.len() - 1 - m] != rounds + 1 && table[m] != rounds + 1 {
                // Pair the teams based on the circle method
                matches[r].push((table[m], table[table.len() - 1 - m]));
            }
        }

        if let Some(last) = table.pop() {
            table.insert(1, last);
        }
    }

    // Retrieve the current league ID
    let league_id = CompetitionModule::default()
        .competition_count
        .load(deps.storage)?;

    // Save rounds and matches to storage
    let mut duration = round_duration;
    let mut match_number = 1u128;
    let mut rounds_count = 0u64;
    for (i, round_pairings) in matches.iter().enumerate() {
        let round_number = i as u64 + 1;
        let mut matches = vec![];
        let expiration = duration.after(&env.block);

        for &(idx1, idx2) in round_pairings {
            MATCHES.save(
                deps.storage,
                (league_id.u128(), round_number, match_number),
                &Match {
                    team_1: team_addresses[idx1 - 1].clone(),
                    team_2: team_addresses[idx2 - 1].clone(),
                    result: None,
                    match_number: Uint128::from(match_number),
                },
            )?;
            matches.push(Uint128::from(match_number));
            match_number += 1;
        }

        ROUNDS.save(
            deps.storage,
            (league_id.u128(), round_number),
            &Round {
                round_number: Uint64::from(round_number),
                matches,
                expiration,
            },
        )?;
        duration = duration.add(round_duration)?;
        rounds_count += 1;
    }

    // Update competition rounds count
    let competition = CompetitionModule::default().competitions.update(
        deps.storage,
        league_id.u128(),
        |maybe_competition| {
            if let Some(mut competition) = maybe_competition {
                competition.extension.rounds = Uint64::from(rounds_count);
                Ok(competition)
            } else {
                Err(StdError::NotFound {
                    kind: "Competition".to_string(),
                })
            }
        },
    )?;

    // Check competition expiration is greater than the last match's expiration + 1 match expiration duration
    let competition_expiration = duration.after(&env.block);
    if competition.expiration < competition_expiration {
        return Err(ContractError::OverflowError(OverflowError::new(
            OverflowOperation::Add,
            competition_expiration,
            competition.expiration,
        )));
    }

    Ok(response
        .add_attribute("round_duration", round_duration.to_string())
        .add_attribute("rounds", rounds_count.to_string()))
}

pub fn process_match(
    deps: DepsMut,
    info: MessageInfo,
    league_id: Uint128,
    round_number: Uint64,
    match_results: Vec<MatchResult>,
) -> Result<Response, ContractError> {
    let league = CompetitionModule::default()
        .competitions
        .load(deps.storage, league_id.u128())?;

    if league.host != info.sender && league.admin_dao != info.sender {
        return Err(ContractError::CompetitionError(
            cw_competition_base::error::CompetitionError::OwnershipError(
                cw_ownable::OwnershipError::NotOwner,
            ),
        ));
    }

    for match_result in match_results {
        let key = (
            league_id.u128(),
            round_number.u64(),
            match_result.match_number.u128(),
        );
        MATCHES.update(deps.storage, key, |x| -> StdResult<_> {
            match x {
                Some(mut m) => {
                    m.result = match_result.result;

                    Ok(m)
                }
                None => Err(StdError::NotFound {
                    kind: "Match".to_string(),
                }),
            }
        })?;
    }

    Ok(Response::new())
}

pub fn update_distribution(
    deps: DepsMut,
    info: MessageInfo,
    distribution: Vec<MemberShare<String>>,
) -> Result<Response, ContractError> {
    let dao = CompetitionModule::default().get_dao(deps.as_ref())?;
    if info.sender != dao {
        return Err(ContractError::CompetitionError(
            cw_competition_base::error::CompetitionError::Unauthorized {},
        ));
    }

    let validated_distribution = validate_distribution(deps.as_ref(), distribution)?;

    DISTRIBUTION.save(deps.storage, &validated_distribution)?;

    Ok(Response::new())
}

pub(crate) fn validate_distribution(
    deps: Deps,
    distribution: Vec<MemberShare<String>>,
) -> StdResult<Vec<MemberShare<Addr>>> {
    distribution
        .iter()
        .map(|x| x.to_validated(deps))
        .collect::<StdResult<Vec<MemberShare<Addr>>>>()
}
