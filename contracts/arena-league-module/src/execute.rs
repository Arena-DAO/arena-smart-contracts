use cosmwasm_std::{
    Addr, Decimal, DepsMut, MessageInfo, Response, StdError, StdResult, Uint128, Uint64,
};
use cw_balance::{Distribution, MemberPercentage};
use itertools::Itertools;
use std::vec;

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
    response: Response,
    teams: Vec<String>,
    distribution: Vec<Decimal>,
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
    if distribution.iter().sum::<Decimal>() != Decimal::one() {
        return Err(ContractError::StdError(StdError::generic_err(
            "The distribution must sum up to 1",
        )));
    }

    // Retrieve the current league's id
    let league_id = CompetitionModule::default()
        .competition_count
        .load(deps.storage)?;

    // Convert teams to addresses
    let team_addresses: Vec<Addr> = teams
        .iter()
        .map(|x| deps.api.addr_validate(x))
        .collect::<StdResult<_>>()?;

    let mut teams_list = (1..=team_count).collect::<Vec<_>>();
    let rounds = if team_count % 2 != 0 {
        teams_list.push(0); // Using 0 as dummy team

        team_count + 1
    } else {
        team_count
    };

    // Split teams into two groups
    let (x, y) = teams_list.split_at(rounds / 2);
    let mut x = x.to_vec();
    let mut y = y.to_vec();

    let mut round_number = 1u64;
    let mut match_number = 1u128;
    for i in 0..rounds - 1 {
        let mut matches = Vec::new();

        // Rotate teams between x and y after the first round
        if i != 0 {
            let first_y = y.remove(0);
            x.insert(1, first_y);
            let last_x = x.pop().unwrap();
            y.push(last_x);
        }

        // Create matches for the round
        for j in 0..x.len() {
            if x[j] != 0 && y[j] != 0 {
                matches.push(Uint128::from(match_number));

                MATCHES.save(
                    deps.storage,
                    (league_id.u128(), round_number, match_number),
                    &Match {
                        team_1: team_addresses[x[j] - 1].clone(), // adjust index for 0-based array
                        team_2: team_addresses[y[j] - 1].clone(),
                        result: None,
                        match_number: Uint128::from(match_number),
                    },
                )?;
                match_number += 1;
            }
        }

        ROUNDS.save(
            deps.storage,
            (league_id.u128(), round_number),
            &Round {
                round_number: Uint64::from(round_number),
                matches,
            },
        )?;
        round_number += 1;
    }

    // Update competition matches and rounds count
    let competition = CompetitionModule::default().competitions.update(
        deps.storage,
        league_id.u128(),
        |maybe_competition| {
            if let Some(mut competition) = maybe_competition {
                competition.extension.rounds = Uint64::from(round_number - 1);
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

    Ok(response
        .add_attribute("rounds", competition.extension.rounds)
        .add_attribute("matches", competition.extension.matches)
        .add_attribute("teams", competition.extension.teams))
}

pub fn process_matches(
    deps: DepsMut,
    info: MessageInfo,
    league_id: Uint128,
    round_number: Uint64,
    match_results: Vec<MatchResult>,
) -> Result<Response, ContractError> {
    // Load the league data from storage
    let mut league = CompetitionModule::default()
        .competitions
        .load(deps.storage, league_id.u128())?;

    // Validate state and authorization
    CompetitionModule::default().inner_validate_auth(&info.sender, &league)?;

    let mut processed_matches = league.extension.processed_matches;
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
                        processed_matches += Uint128::one();
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

    // Check if the processed matches have changed and update the league data accordingly.
    if processed_matches != league.extension.processed_matches {
        let mut new_league = league.clone();
        new_league.extension.processed_matches = processed_matches;

        CompetitionModule::default().competitions.replace(
            deps.storage,
            league_id.u128(),
            Some(&new_league),
            Some(&league),
        )?;

        league.extension.processed_matches = processed_matches;
    }

    let mut response = Response::new();

    // Distribute funds if all matches have been processed.
    if league.extension.processed_matches >= league.extension.matches {
        let mut leaderboard = query::leaderboard(deps.as_ref(), league_id, None)?;

        // Sort the leaderboard based on points.
        leaderboard.sort_by(|x, y| y.points.cmp(&x.points));

        let placements = league.extension.distribution.len();
        let mut placement_members: Vec<Vec<Addr>> = vec![];
        let mut current_placement = 1;

        // Group members into placements based on their points.
        for (i, member_points) in leaderboard.iter().enumerate() {
            if i == 0 {
                // If we are first, then we can just insert into 1st
                placement_members.push(vec![member_points.member.clone()]);
            } else {
                // Check if the previous member is tied on points
                let previous = &leaderboard[i - 1];

                if previous.points == member_points.points {
                    placement_members[current_placement - 1].push(member_points.member.clone());
                } else {
                    // If we have processed all users that can be fit by placement, then exit early
                    // The last percentages will be summed to the end
                    if i >= placements {
                        break;
                    }

                    placement_members.push(vec![member_points.member.clone()]);

                    current_placement += 1;
                }
            }
            // If all placements are found, then break
            if current_placement > placements {
                break;
            }
        }

        // Adjust the distribution of funds based on member placements.
        let mut member_percentages = vec![];

        // Transform the distribution
        let summed_extras: Decimal = league.extension.distribution
            [placement_members.len()..placements]
            .iter()
            .sum();
        let mut distribution = league.extension.distribution[0..placement_members.len()].to_vec();

        let redistributed_percentage_share =
            summed_extras / Decimal::from_ratio(placement_members.len() as u128, Uint128::one());
        for entry in distribution.iter_mut() {
            *entry += redistributed_percentage_share;
        }

        // Generate the member percentages
        let mut remainder_percentage = Decimal::one();

        for i in 0..placement_members.len() {
            let members = &placement_members[i];
            let placement_percentage =
                distribution[i] / Decimal::from_ratio(members.len() as u128, Uint128::one());
            for member in members {
                remainder_percentage -= placement_percentage;

                member_percentages.push(MemberPercentage::<Addr> {
                    addr: member.clone(),
                    percentage: placement_percentage,
                })
            }
        }

        // Increase 1st place by the remainder
        if remainder_percentage > Decimal::zero() {
            member_percentages[0].percentage += remainder_percentage;
        }

        // Load up the custom distribution info
        let config = CompetitionModule::default().config.load(deps.storage)?;

        response = CompetitionModule::default().inner_process(
            deps,
            league,
            Some(Distribution::<Addr> {
                member_percentages,
                remainder_addr: leaderboard[0].member.clone(),
            }),
            config.extension.tax_cw20_msg,
            config.extension.tax_cw721_msg,
        )?;
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
