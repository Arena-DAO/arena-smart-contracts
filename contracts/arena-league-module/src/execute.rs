use cosmwasm_std::{
    Addr, DepsMut, Env, MessageInfo, OverflowError, OverflowOperation, Response, StdError,
    StdResult, Uint128, Uint64,
};
use cw_utils::Duration;
use std::ops::Add;

use crate::{
    contract::CompetitionModule,
    state::{Match, MatchResult, Round, MATCHES, ROUNDS},
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
    // Convert team names to addresses
    let team_addresses: Vec<Addr> = teams
        .iter()
        .map(|name| deps.api.addr_validate(name))
        .collect::<StdResult<_>>()?;
    let team_count = team_addresses.len();

    // Calculate the number of rounds
    let rounds_count = if team_count % 2 == 0 {
        team_count - 1
    } else {
        team_count
    };
    let matches_per_round = (rounds_count + 1) / 2;

    // Generate match pairings for rounds
    let mut team_indexes: Vec<usize> = (1..=rounds_count + 1).collect();
    let mut rounds: Vec<Vec<(usize, usize)>> = Vec::new();
    for _ in 0..rounds_count {
        let round_pairings: Vec<(usize, usize)> = (0..matches_per_round)
            .filter_map(|m| {
                let idx1 = team_indexes[m];
                let idx2 = team_indexes[team_indexes.len() - 1 - m];
                if idx1 < team_count && idx2 < team_count {
                    Some((idx1, idx2))
                } else {
                    None
                }
            })
            .collect();
        rounds.push(round_pairings);
        team_indexes.rotate_right(1);
    }

    // Retrieve the current league ID
    let league_id = CompetitionModule::default()
        .competition_count
        .load(deps.storage)?
        .checked_add(Uint128::one())?;

    // Save rounds and matches to storage
    let mut duration = round_duration;
    let mut match_number = 1u128;
    for (i, round_pairings) in rounds.iter().enumerate() {
        let round_number = i as u64;
        let mut matches = vec![];
        let expiration = duration.after(&env.block);

        for &(idx1, idx2) in round_pairings {
            MATCHES.save(
                deps.storage,
                (league_id.u128(), round_number, match_number),
                &Match {
                    team_1: team_addresses[idx1].clone(),
                    team_2: team_addresses[idx2].clone(),
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
    }

    // Update competition rounds count
    let competition = CompetitionModule::default().competitions.update(
        deps.storage,
        league_id.u128(),
        |maybe_competition| {
            if let Some(mut competition) = maybe_competition {
                competition.extension.rounds = Uint64::from(rounds_count as u64);
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
    env: Env,
    info: MessageInfo,
    league_id: Uint128,
    round_number: Uint64,
    match_number: Uint128,
    result: Option<bool>,
) -> Result<Response, ContractError> {
    let league = CompetitionModule::default()
        .competitions
        .load(deps.storage, league_id.u128())?;

    if league.dao != info.sender && league.admin_dao != info.sender {
        return Err(ContractError::CompetitionError(
            cw_competition_base::error::CompetitionError::OwnershipError(
                cw_ownable::OwnershipError::NotOwner,
            ),
        ));
    }

    let key = (league_id.u128(), round_number.u64(), match_number.u128());
    MATCHES.update(deps.storage, key, |x| -> StdResult<_> {
        match x {
            Some(mut m) => {
                m.result = Some(MatchResult {
                    result,
                    block: env.block,
                });

                Ok(m)
            }
            None => Err(StdError::NotFound {
                kind: "Match".to_string(),
            }),
        }
    })?;

    Ok(Response::new())
}
