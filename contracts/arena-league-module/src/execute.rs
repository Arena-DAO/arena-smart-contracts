use arena_interface::{competition::stats::StatValue, group, ratings::MemberResult};
use cosmwasm_std::{
    ensure_eq, Addr, Decimal, DepsMut, Env, MessageInfo, Order, Response, StdError, StdResult,
    Uint128, Uint64,
};
use cw_balance::{Distribution, MemberPercentage};
use cw_competition_base::error::CompetitionError;
use std::vec;

use crate::{
    contract::CompetitionModule,
    msg::{League, MatchResultMsg, MemberPoints},
    query,
    state::{Match, PointAdjustment, Round, MATCHES, POINT_ADJUSTMENTS, ROUNDS},
    ContractError,
};

pub fn instantiate_rounds(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    ensure_eq!(
        info.sender,
        env.contract.address,
        ContractError::Unauthorized {}
    );

    let league_module = CompetitionModule::default();
    let league_id = league_module.competition_count.load(deps.storage)?;
    let league = league_module
        .competitions
        .load(deps.storage, league_id.u128())?;

    // Convert teams to addresses
    let teams: Vec<Addr> = deps.querier.query_wasm_smart(
        league.group_contract.to_string(),
        &group::QueryMsg::Members {
            start_after: None,
            limit: None,
        },
    )?;

    let team_count = teams.len();

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
                        team_1: teams[x[j] - 1].clone(), // adjust index for 0-based array
                        team_2: teams[y[j] - 1].clone(),
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

    Ok(Response::default()
        .add_attribute("action", "instantiate_rounds")
        .add_attribute("rounds", (round_number - 1).to_string())
        .add_attribute("matches", (match_number - 1).to_string())
        .add_attribute("teams", team_count.to_string()))
}

/// Processes match results for a league, updates ratings, and calculates final distributions if all matches are complete.
///
/// This function performs the following key operations:
/// 1. Validates the sender's authorization to process matches.
/// 2. Updates the match results and tracks processed matches.
/// 3. Prepares rating updates for matches if the league has a category.
/// 4. Updates the league's processed match count.
/// 5. If all matches are complete:
///    a. Calculates the leaderboard with optional stat-based tiebreaking.
///    b. Groups members into placements based on points and tiebreakers.
///    c. Calculates the final distribution of rewards.
///    d. Processes the competition results.
///
/// The stat-based tiebreaking is applied only if stat types are defined for the league.
/// The function uses the priority index to ensure stat types are considered in the correct order.
pub fn process_matches(
    deps: DepsMut,
    info: MessageInfo,
    league_id: Uint128,
    round_number: Uint64,
    match_results: Vec<MatchResultMsg>,
) -> Result<Response, ContractError> {
    // Load the league data from storage
    let mut league = CompetitionModule::default()
        .competitions
        .load(deps.storage, league_id.u128())?;

    // Validate state and authorization
    CompetitionModule::default().inner_validate_auth(&info.sender, &league, false)?;

    let mut processed_matches = league.extension.processed_matches;
    let mut member_results = vec![];

    // Process each match result
    for match_result in match_results {
        let key = (
            league_id.u128(),
            round_number.u64(),
            match_result.match_number.u128(),
        );
        MATCHES.update(deps.storage, key, |x| -> Result<_, ContractError> {
            match x {
                Some(mut m) => {
                    if m.result.is_none() {
                        processed_matches += Uint128::one();

                        if league.category_id.is_some() {
                            // Prepare rating updates (only handled once per match)
                            let (member_result_1, member_result_2) = match match_result.match_result
                            {
                                crate::state::MatchResult::Team1 => {
                                    (Decimal::one(), Decimal::zero())
                                }
                                crate::state::MatchResult::Team2 => {
                                    (Decimal::zero(), Decimal::one())
                                }
                                crate::state::MatchResult::Draw => {
                                    (Decimal::percent(50), Decimal::percent(50))
                                }
                            };

                            member_results.push((
                                MemberResult {
                                    addr: m.team_1.clone(),
                                    result: member_result_1,
                                },
                                MemberResult {
                                    addr: m.team_2.clone(),
                                    result: member_result_2,
                                },
                            ));
                        }
                    }
                    m.result = Some(match_result.match_result);
                    Ok(m)
                }
                None => Err(ContractError::StdError(StdError::NotFound {
                    kind: "Match".to_string(),
                })),
            }
        })?;
    }

    // Trigger rating adjustments if applicable
    let mut sub_msgs = vec![];
    if let Some(category_id) = league.category_id {
        if CompetitionModule::default().query_is_dao_member(
            deps.as_ref(),
            &league.host,
            league.start_height,
        ) {
            sub_msgs.push(CompetitionModule::default().trigger_rating_adjustment(
                deps.storage,
                category_id,
                member_results,
            )?);
        }
    }

    // Update the league's processed matches count if changed
    if processed_matches != league.extension.processed_matches {
        let mut updated_league = league.clone();
        updated_league.extension.processed_matches = processed_matches;

        CompetitionModule::default().competitions.replace(
            deps.storage,
            league_id.u128(),
            Some(&updated_league),
            Some(&league),
        )?;

        league.extension.processed_matches = processed_matches;
    }

    let mut response = Response::new();

    // Process final results if all matches have been completed
    if league.extension.processed_matches >= league.extension.matches {
        response = process_final_results(deps, &league, league_id)?;
    }

    Ok(response
        .add_attribute("action", "process_matches")
        .add_attribute("processed_matches", processed_matches.to_string())
        .add_submessages(sub_msgs))
}

fn process_final_results(
    deps: DepsMut,
    league: &League,
    league_id: Uint128,
) -> Result<Response, CompetitionError> {
    let mut leaderboard = query::leaderboard(deps.as_ref(), league_id, None)?;

    // Fetch and sort stat types by priority
    let mut stat_types: Vec<_> = CompetitionModule::default()
        .stat_types
        .prefix(league_id.u128())
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    stat_types.sort_by(|a, b| {
        a.1.tie_breaker_priority
            .unwrap_or(u8::MAX)
            .cmp(&b.1.tie_breaker_priority.unwrap_or(u8::MAX))
    });

    // Define a comparison function that considers both points and stats
    let compare_members = |a: &MemberPoints, b: &MemberPoints| {
        b.points.cmp(&a.points).then_with(|| {
            for (_, stat_type) in &stat_types {
                let (a_stat, b_stat) = match &stat_type.aggregation_type {
                    Some(_) => (
                        CompetitionModule::default()
                            .inner_aggregate(deps.as_ref(), league_id, &a.member, stat_type)
                            .ok(),
                        CompetitionModule::default()
                            .inner_aggregate(deps.as_ref(), league_id, &b.member, stat_type)
                            .ok(),
                    ),
                    None => (
                        CompetitionModule::default()
                            .stats
                            .may_load(deps.storage, (league_id.u128(), &a.member, &stat_type.name))
                            .ok()
                            .flatten(),
                        CompetitionModule::default()
                            .stats
                            .may_load(deps.storage, (league_id.u128(), &b.member, &stat_type.name))
                            .ok()
                            .flatten(),
                    ),
                };
                dbg!(a_stat.clone());
                dbg!(b_stat.clone());
                if let (Some(a_val), Some(b_val)) = (a_stat, b_stat) {
                    let cmp = compare_stat_values(&a_val, &b_val, stat_type.is_beneficial);
                    if cmp != std::cmp::Ordering::Equal {
                        return cmp;
                    }
                }
            }
            std::cmp::Ordering::Equal
        })
    };

    // Sort the leaderboard using the comparison function
    leaderboard.sort_by(compare_members);

    let placements = league.extension.distribution.len();
    let mut placement_members: Vec<Vec<Addr>> = vec![];

    // Group members into placements based on their points and tiebreakers
    for (i, member_points) in leaderboard.iter().enumerate() {
        if i == 0 {
            placement_members.push(vec![member_points.member.clone()]);
        } else {
            let previous = &leaderboard[i - 1];
            if compare_members(previous, member_points) == std::cmp::Ordering::Equal {
                placement_members
                    .last_mut()
                    .unwrap()
                    .push(member_points.member.clone());
            } else {
                if placement_members.len() >= placements {
                    break;
                }
                placement_members.push(vec![member_points.member.clone()]);
            }
        }
    }

    // Calculate the final distribution
    let mut member_percentages = vec![];
    let summed_extras: Decimal = league.extension.distribution[placement_members.len()..placements]
        .iter()
        .sum();
    let mut distribution = league.extension.distribution[0..placement_members.len()].to_vec();
    let redistributed_percentage_share = summed_extras.checked_div(Decimal::from_ratio(
        placement_members.len() as u128,
        Uint128::one(),
    ))?;

    for entry in distribution.iter_mut() {
        *entry = entry.checked_add(redistributed_percentage_share)?;
    }

    let mut remainder_percentage = Decimal::one();
    for (i, members) in placement_members.iter().enumerate() {
        let placement_percentage = distribution[i]
            .checked_div(Decimal::from_ratio(members.len() as u128, Uint128::one()))?;
        for member in members {
            remainder_percentage = remainder_percentage.checked_sub(placement_percentage)?;
            member_percentages.push(MemberPercentage::<Addr> {
                addr: member.clone(),
                percentage: placement_percentage,
            });
        }
    }

    if remainder_percentage > Decimal::zero() {
        member_percentages[0].percentage = member_percentages[0]
            .percentage
            .checked_add(remainder_percentage)?;
    }

    // Process the competition results
    CompetitionModule::default().inner_process(
        deps,
        league,
        Some(Distribution::<Addr> {
            member_percentages,
            remainder_addr: leaderboard[0].member.clone(),
        }),
    )
}

// Helper function to compare stat values
fn compare_stat_values(a: &StatValue, b: &StatValue, is_beneficial: bool) -> std::cmp::Ordering {
    let ord = match (a, b) {
        (StatValue::Bool(a), StatValue::Bool(b)) => a.cmp(b),
        (StatValue::Decimal(a), StatValue::Decimal(b)) => a.cmp(b),
        (StatValue::Uint(a), StatValue::Uint(b)) => a.cmp(b),
        _ => std::cmp::Ordering::Equal,
    };
    if is_beneficial {
        ord.reverse()
    } else {
        ord
    }
}

pub fn update_distribution(
    deps: DepsMut,
    info: MessageInfo,
    league_id: Uint128,
    distribution: Vec<Decimal>,
) -> Result<Response, ContractError> {
    let league = CompetitionModule::default()
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
    if distribution.iter().sum::<Decimal>() != Decimal::one() {
        return Err(ContractError::StdError(StdError::generic_err(
            "The distribution must sum up to 1",
        )));
    }
    let mut updated_league = league.clone();
    updated_league.extension.distribution = distribution;

    CompetitionModule::default().competitions.replace(
        deps.storage,
        league_id.u128(),
        Some(&updated_league),
        Some(&league),
    )?;

    Ok(Response::new()
        .add_attribute("action", "update_distribution")
        .add_attribute(
            "distribution",
            format!("{:#?}", updated_league.extension.distribution),
        ))
}

pub fn add_point_adjustments(
    deps: DepsMut,
    info: MessageInfo,
    league_id: Uint128,
    addr: String,
    mut point_adjustments: Vec<PointAdjustment>,
) -> Result<Response, ContractError> {
    let league = CompetitionModule::default()
        .competitions
        .load(deps.storage, league_id.u128())?;

    // Validate state and authorization
    CompetitionModule::default().inner_validate_auth(&info.sender, &league, true)?;

    if point_adjustments.iter().any(|x| x.amount.is_zero()) {
        return Err(ContractError::StdError(StdError::generic_err(
            "Cannot adjust points by 0",
        )));
    }

    let addr = deps.api.addr_validate(&addr)?;

    // iterate through max 2 sets of matches to find addr
    let mut is_found = false;
    'outer: for i in 1..=2 {
        if let Some(round) = ROUNDS.may_load(deps.storage, (league_id.u128(), i as u64))? {
            for j in round.matches {
                if let Some(m) = MATCHES.may_load(
                    deps.storage,
                    (league_id.u128(), round.round_number.u64(), j.u128()),
                )? {
                    if m.team_1 == addr || m.team_2 == addr {
                        is_found = true;

                        break 'outer;
                    }
                }
            }
        }
    }

    if !is_found {
        return Err(ContractError::StdError(StdError::generic_err(format!(
            "{} is not a member of the competition",
            addr
        ))));
    }

    POINT_ADJUSTMENTS.update(
        deps.storage,
        (league_id.u128(), &addr),
        |x| -> StdResult<_> {
            match x {
                Some(mut previous_adjustments) => {
                    previous_adjustments.append(&mut point_adjustments);

                    Ok(previous_adjustments)
                }
                None => Ok(point_adjustments),
            }
        },
    )?;

    Ok(Response::new().add_attribute("action", "add_point_adjustments"))
}
