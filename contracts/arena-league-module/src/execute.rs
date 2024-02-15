use cosmwasm_std::{
    Addr, Decimal, DepsMut, Env, MessageInfo, OverflowError, OverflowOperation, Response, StdError,
    StdResult, Uint128, Uint64,
};
use cw_balance::MemberPercentage;
use cw_utils::Duration;
use itertools::Itertools;
use std::{ops::Add, vec};

use crate::{
    contract::CompetitionModule,
    msg::MatchResult,
    query,
    state::{Match, Round, MATCHES, ROUNDS},
    ContractError,
};

#[allow(clippy::too_many_arguments)]
pub fn instantiate_rounds(
    deps: DepsMut,
    env: Env,
    response: Response,
    teams: Vec<String>,
    distribution: Vec<Decimal>,
    round_duration: Duration,
) -> Result<Response, ContractError> {
    let team_count = teams.len();
    if team_count < 2 {
        return Err(ContractError::StdError(StdError::GenericErr {
            msg: "At least 2 teams should be provided".to_string(),
        }));
    }
    if teams.iter().unique().count() != team_count {
        return Err(ContractError::StdError(StdError::GenericErr {
            msg: "Teams should not contain duplicates".to_string(),
        }));
    }
    if distribution.len() > team_count {
        return Err(ContractError::StdError(StdError::GenericErr {
            msg: "Cannot have a distribution size bigger than the teams size".to_string(),
        }));
    }

    // Convert teams to addresses
    let team_addresses: Vec<Addr> = teams
        .iter()
        .map(|x| deps.api.addr_validate(x))
        .collect::<StdResult<_>>()?;

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

    // Update competition matches and rounds count
    let competition = CompetitionModule::default().competitions.update(
        deps.storage,
        league_id.u128(),
        |maybe_competition| {
            if let Some(mut competition) = maybe_competition {
                competition.extension.rounds = Uint64::from(rounds_count);
                competition.extension.matches = Uint128::from(match_number - 1);
                competition.extension.teams = Uint64::from(team_count as u64);
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

pub fn process_matches(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    league_id: Uint128,
    round_number: Uint64,
    match_results: Vec<MatchResult>,
) -> Result<Response, ContractError> {
    let mut league = CompetitionModule::default()
        .competitions
        .load(deps.storage, league_id.u128())?;

    if league.host != info.sender && league.admin_dao != info.sender {
        return Err(ContractError::CompetitionError(
            cw_competition_base::error::CompetitionError::OwnershipError(
                cw_ownable::OwnershipError::NotOwner,
            ),
        ));
    }

    let round = ROUNDS.load(deps.storage, (league_id.u128(), round_number.u64()))?;
    // Limit when results can be set to prevent malicious vote spam
    if !round.expiration.is_expired(&env.block) {
        return Err(ContractError::NotExpired {
            expiration: round.expiration,
        });
    }

    for match_result in match_results {
        let key = (
            league_id.u128(),
            round_number.u64(),
            match_result.match_number.u128(),
        );
        MATCHES.update(deps.storage, key, |x| -> Result<_, ContractError> {
            match x {
                Some(mut m) => {
                    // Only the admin dao can override an existing result
                    if m.result.is_some() && league.admin_dao != info.sender {
                        return Err(ContractError::CompetitionError(
                            cw_competition_base::error::CompetitionError::OwnershipError(
                                cw_ownable::OwnershipError::NotOwner,
                            ),
                        ));
                    } else if m.result.is_none() {
                        league.extension.processed_matches = league
                            .extension
                            .processed_matches
                            .checked_add(Uint128::one())?;
                    }
                    m.result = match_result.result;

                    Ok(m)
                }
                None => Err(ContractError::StdError(StdError::NotFound {
                    kind: "Match".to_string(),
                })),
            }
        })?;
    }

    let mut response = Response::new();

    if let Some(_escrow) = league.escrow {
        if league.extension.processed_matches >= league.extension.matches {
            // Distribute funds if we have processed all of the matches
            let mut leaderboard = query::leaderboard(deps.as_ref(), league_id, None)?;

            leaderboard.sort_by(|x, y| y.points.cmp(&x.points));

            let mut distribution = vec![];

            for (i, x) in league.extension.distribution.iter().enumerate() {
                distribution.push(MemberPercentage::<String> {
                    addr: leaderboard[i].member.to_string(),
                    percentage: *x,
                })
            }

            let config = CompetitionModule::default().config.load(deps.storage)?;

            response = CompetitionModule::default().execute_process_competition(
                deps,
                info,
                league_id,
                distribution,
                config.extension.cw20_msg,
                config.extension.cw721_msg,
            )?;
        }
    }

    Ok(response.add_attribute("action", "process_matches"))
}

pub fn update_distribution(
    deps: DepsMut,
    info: MessageInfo,
    league_id: Uint128,
    distribution: Vec<Decimal>,
) -> Result<Response, ContractError> {
    let mut league = CompetitionModule::default()
        .competitions
        .load(deps.storage, league_id.u128())?;

    if info.sender != league.admin_dao {
        return Err(ContractError::CompetitionError(
            cw_competition_base::error::CompetitionError::Unauthorized {},
        ));
    }
    if distribution.len() as u64 > league.extension.teams.u64() {
        return Err(ContractError::StdError(StdError::GenericErr {
            msg: "Cannot have a distribution size bigger than the teams size".to_string(),
        }));
    }

    league.extension.distribution = distribution.clone();

    CompetitionModule::default()
        .competitions
        .save(deps.storage, league_id.u128(), &league)?;

    Ok(Response::new()
        .add_attribute("action", "update_distribution")
        .add_attribute("distribution", format!("{:#?}", distribution)))
}
