use crate::contract::CompetitionModule;
use crate::msg::{MatchResultMsg, Tournament};
use crate::state::{EliminationType, Match, MatchResult, MATCHES};
use crate::{ContractError, NestedArray};
use cosmwasm_std::{Addr, MessageInfo, StdError, Storage};
use cosmwasm_std::{DepsMut, Response, StdResult, Uint128};
use cw_balance::{Distribution, MemberPercentage};
use itertools::Itertools;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::iter::repeat;

pub fn instantiate_tournament(
    deps: DepsMut,
    response: Response,
    teams: Vec<String>,
    elimination_type: EliminationType,
) -> Result<Response, ContractError> {
    let competition_module = CompetitionModule::default();
    let tournament_id = competition_module.competition_count.load(deps.storage)?;

    // Convert teams to addresses
    let teams: Vec<Addr> = teams
        .iter()
        .map(|x| deps.api.addr_validate(x))
        .collect::<StdResult<_>>()?;

    // Single Elimination Bracket
    if let EliminationType::SingleElimination {
        play_third_place_match,
    } = elimination_type
    {
        generate_single_elimination_bracket(
            deps,
            &teams,
            tournament_id.u128(),
            play_third_place_match,
        )?;
    } else {
        // Double Elimination Bracket
        generate_double_elimination_bracket(deps, &teams, tournament_id.u128())?;
    }

    Ok(response
        .add_attribute("action", "instantiate_tournament")
        .add_attribute("tournament_id", tournament_id.to_string()))
}

fn generate_matches(
    nested: &NestedArray<usize>,
    teams: &[Addr],
    matches: &mut HashMap<u128, Match>,
    layer_map: &mut BTreeMap<usize, BTreeSet<u128>>,
) {
    let mut queue = VecDeque::new();
    queue.push_back((nested, None::<Uint128>, 1));

    while let Some((current_nested, parent_match_number, layer)) = queue.pop_front() {
        match current_nested {
            NestedArray::Single(indices) => {
                let team_1 = teams.get(indices[0]).cloned();
                let team_2 = teams.get(indices[1]).cloned();

                // Handle byes if one of the teams is missing
                if team_1.is_none() || team_2.is_none() {
                    // Assuming that if a team is missing, the other team gets a bye
                    let advancing_team = team_1.or(team_2);

                    // Handle advancing teams
                    if let Some(team) = advancing_team {
                        if let Some(parent_num) = parent_match_number {
                            if let Some(match_) = matches.get_mut(&parent_num.u128()) {
                                // Update the match with the advancing team
                                if match_.team_1.is_none() {
                                    match_.team_1 = Some(team);
                                } else {
                                    match_.team_2 = Some(team);
                                }

                                // Insert into layer
                                let layer_values = layer_map.entry(layer).or_default();
                                layer_values.insert(match_.match_number.u128());

                                // Remove from previous layer
                                let previous_layer_values =
                                    layer_map.get_mut(&(layer - 1)).unwrap();
                                previous_layer_values.remove(&parent_num.u128());
                            }
                        }
                    }
                } else {
                    // No byes, create match normally
                    create_match(
                        team_1,
                        team_2,
                        parent_match_number,
                        matches,
                        layer_map,
                        layer,
                        None,
                    );
                }
            }
            NestedArray::Nested(nested_vec) => {
                let match_number = create_match(
                    None,
                    None,
                    parent_match_number,
                    matches,
                    layer_map,
                    layer,
                    None,
                );

                // Enqueue all nested elements
                for nested_element in nested_vec {
                    queue.push_back((nested_element, Some(match_number), layer + 1));
                }
            }
        }
    }

    // Setup to find and potentially remove the second-to-last entry if empty
    let keys = layer_map.keys().copied().collect_vec();
    let total_keys = keys.len();

    // Check if there are at least two keys to process
    if total_keys >= 2 {
        let second_last_key = keys[total_keys - 2];
        let last_key = keys[total_keys - 1];

        // Check if the second-to-last entry is empty and process
        if layer_map
            .get(&second_last_key)
            .map_or(false, BTreeSet::is_empty)
        {
            if let Some(last_value) = layer_map.remove(&last_key) {
                layer_map.insert(second_last_key, last_value);
            }
        }
    }
}

fn create_match(
    team_1: Option<Addr>,
    team_2: Option<Addr>,
    parent_match_number: Option<Uint128>,
    matches: &mut HashMap<u128, Match>,
    layer_map: &mut BTreeMap<usize, BTreeSet<u128>>,
    layer: usize,
    is_losers_bracket: Option<bool>,
) -> Uint128 {
    let match_number = Uint128::new(matches.len() as u128 + 1); // Assuming match numbers are sequential
    let match_ = Match {
        match_number,
        team_1,
        team_2,
        result: None,
        next_match_winner: parent_match_number,
        next_match_loser: None,
        is_losers_bracket,
    };

    matches.insert(match_number.u128(), match_);

    let entry = layer_map.entry(layer).or_default();
    entry.insert(match_number.u128());

    match_number
}

fn generate_single_elimination_bracket(
    deps: DepsMut,
    teams: &[Addr],
    tournament_id: u128,
    play_third_place_match: bool,
) -> StdResult<()> {
    let mut matches = HashMap::new();
    let mut layer_map = BTreeMap::new();

    {
        // Get the bracket structure
        let n = teams.len().next_power_of_two();
        let sorted_indexes = NestedArray::Single((0..n).collect_vec()).nest();

        generate_matches(&sorted_indexes, teams, &mut matches, &mut layer_map);
    }

    // Optionally add a third place match
    if play_third_place_match {
        // Shift the final up
        if let Some(value) = layer_map.remove(&1) {
            layer_map.insert(0, value);
        }

        let third_place_match_number = create_match(
            None,
            None,
            None,
            &mut matches,
            &mut layer_map,
            1,
            Some(true),
        );

        if let Some(match_) = matches.get_mut(&2) {
            match_.next_match_loser = Some(third_place_match_number);
        }
        if let Some(match_) = matches.get_mut(&3) {
            match_.next_match_loser = Some(third_place_match_number);
        }
    }

    // Save matches
    save_matches(&mut matches, layer_map, tournament_id, deps.storage)
}

// Updates the match ordering and saves them to storage
fn save_matches(
    matches: &mut HashMap<u128, Match>,
    layer_map: BTreeMap<usize, BTreeSet<u128>>,
    tournament_id: u128,
    storage: &mut dyn Storage,
) -> StdResult<()> {
    // Fix match ordering and save
    // The final should be the last match number
    let mut updates = HashMap::new();
    let mut match_number = Uint128::zero();

    // First pass to update match numbers
    for layer in layer_map.keys().rev() {
        for old_match_number in layer_map.get(layer).unwrap() {
            match_number += Uint128::one();
            updates.insert(old_match_number, match_number);
        }
    }

    // Second pass to update paths and save
    for match_info in matches.values_mut() {
        if let Some(update) = updates.get(&match_info.match_number.u128()) {
            match_info.match_number = *update;
        }

        if let Some(next_match_winner) = match_info.next_match_winner {
            if let Some(update) = updates.get(&next_match_winner.u128()) {
                match_info.next_match_winner = Some(*update);
            }
        }
        if let Some(next_match_loser) = match_info.next_match_loser {
            if let Some(update) = updates.get(&next_match_loser.u128()) {
                match_info.next_match_loser = Some(*update);
            }
        }

        // Save
        MATCHES.save(
            storage,
            (tournament_id, match_info.match_number.u128()),
            match_info,
        )?;
    }

    Ok(())
}

fn generate_double_elimination_bracket(
    deps: DepsMut,
    teams: &[Addr],
    tournament_id: u128,
) -> StdResult<()> {
    let mut matches = HashMap::new();
    let mut layer_map = BTreeMap::new();

    {
        // Get the bracket structure
        let n = teams.len().next_power_of_two();
        let sorted_indexes = NestedArray::Single((0..n).collect_vec()).nest();

        generate_matches(&sorted_indexes, teams, &mut matches, &mut layer_map);
    }

    // Once we have the winner's bracket, we can generate the loser's bracket + additional matches
    let mut next_layer_matches = BTreeSet::new();
    let layers = layer_map.keys().rev().cloned().collect_vec();
    for layer in layers {
        // Remove matches from previous layer
        if let Some(layer_matches) = layer_map.get_mut(&(layer + 1)) {
            for match_number in next_layer_matches.iter() {
                layer_matches.remove(match_number);
            }
        }
        // Add any new matches to the layer
        layer_map.get_mut(&layer).map(|x| {
            x.append(&mut next_layer_matches);
            Some(())
        });
        let layer_matches = layer_map[&layer].len();
        let n = layer_matches.next_power_of_two();
        let mut adjusted_matches = NestedArray::Single(
            repeat(0u128)
                .take(n - layer_matches)
                .interleave(layer_map[&layer].iter().rev().cloned())
                .collect_vec(),
        )
        .nest_flat();

        // Create the base matches
        let mut bye_match = None;
        let mut prev_match = None;
        while !adjusted_matches.is_empty() {
            let mut first = adjusted_matches.pop_front().unwrap_or(0);
            let mut second = adjusted_matches.pop_front().unwrap_or(0);

            // If either of the matches are byes, then queue up the match for pairing
            let mut is_bye = (first == 0) ^ (second == 0);
            if is_bye {
                if let Some(match_) = bye_match {
                    // If a bye already exists, then let's just pair up these two byes
                    if first == 0 {
                        first = match_;
                    } else {
                        second = match_;
                    }

                    bye_match = None; // Clear the stored bye match
                    is_bye = false;
                }
            }

            // Create match
            let match_number = create_match(
                None,
                None,
                bye_match.map(Uint128::new),
                &mut matches,
                &mut layer_map,
                layer,
                Some(true),
            );

            if let Some(bye) = bye_match {
                next_layer_matches.insert(bye);
            }

            if let Some(prev) = prev_match {
                // Handle the case where the bye happens in the last node
                if bye_match.is_none() {
                    if is_bye {
                        if let Some(match_) = matches.get_mut(&prev) {
                            match_.next_match_winner = Some(match_number);
                        }
                    } else {
                        next_layer_matches.insert(prev);
                    }
                    next_layer_matches.insert(match_number.u128());
                }

                prev_match = None;
                bye_match = None;
            } else {
                prev_match = Some(match_number.u128());

                if is_bye {
                    bye_match = Some(match_number.u128());
                }
            }

            // Update previous matches for next match links
            if let Some(match_) = matches.get_mut(&first) {
                if match_.is_losers_bracket.unwrap_or(false) {
                    match_.next_match_winner = Some(match_number);
                } else {
                    match_.next_match_loser = Some(match_number);
                }
            }
            if let Some(match_) = matches.get_mut(&second) {
                if match_.is_losers_bracket.unwrap_or(false) {
                    match_.next_match_winner = Some(match_number);
                } else {
                    match_.next_match_loser = Some(match_number);
                }
            }
        }
    }

    // Create a final layer
    {
        let first_layer = layer_map.get(&1).unwrap();
        let mut losers_final = 0;
        let first_layer_keys = first_layer.iter().copied().collect_vec();

        for key in first_layer_keys.iter() {
            if *key != 0 {
                if let Some(match_) = matches.get(key) {
                    if match_.next_match_winner.is_none() {
                        losers_final = match_.match_number.u128();
                    }
                }
            }
        }

        layer_map.insert(0, BTreeSet::from([first_layer_keys[0], losers_final]));
        if let Some(layer) = layer_map.get_mut(&1) {
            layer.remove(&first_layer_keys[0]);
            layer.remove(&losers_final);
        }
    }

    // Create the grand finale
    let final_match_number = create_match(None, None, None, &mut matches, &mut layer_map, 0, None);

    let last_layer = layer_map[&0].iter().collect_vec();
    if let Some(match_) = matches.get_mut(last_layer[0]) {
        match_.next_match_winner = Some(final_match_number);
    }
    if let Some(match_) = matches.get_mut(last_layer[1]) {
        match_.next_match_winner = Some(final_match_number);
    }

    // The rebuttal match will be added dynamically on final processing

    save_matches(&mut matches, layer_map, tournament_id, deps.storage)
}

pub fn process_matches(
    deps: DepsMut,
    info: MessageInfo,
    tournament_id: Uint128,
    match_results: Vec<MatchResultMsg>,
) -> Result<Response, ContractError> {
    // Validate authorization
    let competition_module = CompetitionModule::default();
    let mut tournament = competition_module
        .competitions
        .load(deps.storage, tournament_id.u128())?;
    competition_module.inner_validate_auth(&info.sender, &tournament)?;

    // Prepare updates for the next matches
    let mut updates = Vec::new();
    let mut newly_processed_matches = 0;

    // Process each match result
    for result in match_results {
        let mut match_ = MATCHES.update(
            deps.storage,
            (tournament_id.u128(), result.match_number.u128()),
            |match_info| -> StdResult<_> {
                let mut match_info = match_info.ok_or_else(|| {
                    StdError::generic_err(format!("Match number {} not found", result.match_number))
                })?;

                if match_info.team_1.is_none() || match_info.team_2.is_none() {
                    return Err(StdError::generic_err("Match is not populated yet"));
                }

                let previous_result = match match_info.result {
                    Some(previous_result) => Some(match previous_result {
                        MatchResult::Team1 => match_info.team_1.clone(),
                        MatchResult::Team2 => match_info.team_2.clone(),
                    }),
                    None => {
                        newly_processed_matches += 1;
                        None
                    }
                }
                .flatten();

                match_info.result = Some(result.match_result.clone());

                // Determine the winning and losing teams
                let (winner_team, loser_team) = match result.match_result {
                    MatchResult::Team1 => (match_info.team_1.clone(), match_info.team_2.clone()),
                    MatchResult::Team2 => (match_info.team_2.clone(), match_info.team_1.clone()),
                };

                // Update the next match with the losing team in double elimination and third place matches
                if let Some(next_match_loser) = match_info.next_match_loser {
                    updates.push((
                        next_match_loser,
                        loser_team.clone(),
                        previous_result.clone(),
                    ));
                }

                // Update the next match with the winning team
                if let Some(next_match_winner) = match_info.next_match_winner {
                    updates.push((next_match_winner, winner_team.clone(), previous_result));
                }

                Ok(match_info)
            },
        )?;

        // If we're processing the last match of a double elim tournament, then we should add a rebuttal match if the loser's bracket won
        if match_.match_number == tournament.extension.total_matches
            && matches!(
                tournament.extension.elimination_type,
                EliminationType::DoubleElimination
            )
            && match_.is_losers_bracket.is_none()
        {
            let loser_final = MATCHES.load(
                deps.storage,
                (tournament_id.u128(), match_.match_number.u128() - 1),
            )?;

            let grand_final_winner = match match_.result.as_ref().unwrap() {
                MatchResult::Team1 => match_.team_1.clone(),
                MatchResult::Team2 => match_.team_2.clone(),
            }
            .unwrap();

            let loser_final_winner = match loser_final.result.unwrap() {
                MatchResult::Team1 => loser_final.team_1,
                MatchResult::Team2 => loser_final.team_2,
            }
            .unwrap();

            if grand_final_winner == loser_final_winner {
                tournament.extension.total_matches += Uint128::one();
                MATCHES.save(
                    deps.storage,
                    (
                        tournament_id.u128(),
                        tournament.extension.total_matches.u128(),
                    ),
                    &Match {
                        match_number: tournament.extension.total_matches,
                        team_1: match_.team_1.clone(),
                        team_2: match_.team_2.clone(),
                        result: None,
                        next_match_winner: None,
                        next_match_loser: None,
                        is_losers_bracket: Some(true),
                    },
                )?;

                match_.next_match_loser = Some(tournament.extension.total_matches);
                match_.next_match_winner = Some(tournament.extension.total_matches);

                MATCHES.save(
                    deps.storage,
                    (tournament_id.u128(), match_.match_number.u128()),
                    &match_,
                )?;
            }
        }
    }

    // Apply updates to the next matches
    let mut index = 0;
    while index < updates.len() {
        let (next_match_number, team, previous) = updates[index].clone();
        index += 1;

        MATCHES.update(
            deps.storage,
            (tournament_id.u128(), next_match_number.u128()),
            |next_match| -> StdResult<_> {
                let mut next_match = next_match.ok_or_else(|| {
                    StdError::generic_err(format!(
                        "Next match number {} not found",
                        next_match_number
                    ))
                })?;

                if next_match.team_1 == previous {
                    next_match.team_1.clone_from(&team);
                } else {
                    next_match.team_2.clone_from(&team);
                }

                if next_match.result.is_some() {
                    // Determine the winning and losing teams for the next match
                    let (winner_team, loser_team) = match next_match.result {
                        None => (None, None), // This case should not occur but kept for completeness
                        Some(ref result) => match result {
                            MatchResult::Team1 => {
                                (next_match.team_1.clone(), next_match.team_2.clone())
                            }
                            MatchResult::Team2 => {
                                (next_match.team_2.clone(), next_match.team_1.clone())
                            }
                        },
                    };

                    // Update the next match with the losing team in double elimination and third place matches
                    if let Some(next_match_loser) = next_match.next_match_loser {
                        if loser_team == previous {
                            updates.push((next_match_loser, loser_team.clone(), previous.clone()));
                        }
                    }

                    // Update the next match with the winning team
                    if let Some(next_match_winner) = next_match.next_match_winner {
                        if winner_team == previous {
                            updates.push((next_match_winner, winner_team.clone(), previous));
                        }
                    }
                }

                Ok(next_match)
            },
        )?;
    }

    // Update processed matches count
    tournament.extension.processed_matches += Uint128::new(newly_processed_matches);

    competition_module
        .competitions
        .save(deps.storage, tournament_id.u128(), &tournament)?;

    // Trigger distribution if all matches are processed
    let response = if tournament.extension.processed_matches >= tournament.extension.total_matches {
        // Trigger the distribution logic here
        trigger_distribution(deps, tournament)?
    } else {
        Response::new()
    };

    Ok(response.add_attribute("action", "process_matches"))
}

fn trigger_distribution(deps: DepsMut, tournament: Tournament) -> Result<Response, ContractError> {
    let mut placements: Vec<Addr> = Vec::new();

    match tournament.extension.elimination_type {
        EliminationType::SingleElimination {
            play_third_place_match,
        } => {
            // Load the final match
            let final_match = MATCHES.load(
                deps.storage,
                (
                    tournament.id.u128(),
                    tournament.extension.total_matches.u128(),
                ),
            )?;
            let (first_place, second_place) = match final_match.result.as_ref().unwrap() {
                MatchResult::Team1 => (
                    final_match.team_1.as_ref().unwrap(),
                    final_match.team_2.as_ref().unwrap(),
                ),
                MatchResult::Team2 => (
                    final_match.team_2.as_ref().unwrap(),
                    final_match.team_1.as_ref().unwrap(),
                ),
            };
            placements.push(first_place.to_owned());
            placements.push(second_place.to_owned());

            if play_third_place_match {
                // Load the third place match
                let third_place_match = MATCHES.load(
                    deps.storage,
                    (
                        tournament.id.u128(),
                        tournament.extension.total_matches.u128() - 1,
                    ),
                )?;
                let (third_place, fourth_place) = match third_place_match.result.as_ref().unwrap() {
                    MatchResult::Team1 => (
                        third_place_match.team_1.as_ref().unwrap(),
                        third_place_match.team_2.as_ref().unwrap(),
                    ),
                    MatchResult::Team2 => (
                        third_place_match.team_2.as_ref().unwrap(),
                        third_place_match.team_1.as_ref().unwrap(),
                    ),
                };
                placements.push(third_place.to_owned());
                placements.push(fourth_place.to_owned());
            }
        }
        EliminationType::DoubleElimination => {
            // Load the final matches
            // [Rebuttal?, final, and losers final]
            let final_matches = MATCHES
                .prefix(tournament.id.u128())
                .range(deps.storage, None, None, cosmwasm_std::Order::Descending)
                .take(3)
                .map(|x| x.map(|y| y.1))
                .collect::<StdResult<Vec<_>>>()?;

            let (final_match, losers_final_match) = match final_matches[0].is_losers_bracket {
                Some(_) => (&final_matches[0], &final_matches[2]),
                None => (&final_matches[0], &final_matches[1]),
            };

            let (first_place, second_place) = match final_match.result.as_ref().unwrap() {
                MatchResult::Team1 => (
                    final_match.team_1.as_ref().unwrap(),
                    final_match.team_2.as_ref().unwrap(),
                ),
                MatchResult::Team2 => (
                    final_match.team_2.as_ref().unwrap(),
                    final_match.team_1.as_ref().unwrap(),
                ),
            };
            placements.push(first_place.to_owned());
            placements.push(second_place.to_owned());

            let third_place = match losers_final_match.result.as_ref().unwrap() {
                MatchResult::Team1 => losers_final_match.team_2.as_ref().unwrap(),
                MatchResult::Team2 => losers_final_match.team_1.as_ref().unwrap(),
            };
            placements.push(third_place.to_owned());
        }
    }

    // Implement the distribution logic here using the `placements` vector
    let mut member_percentages: Vec<MemberPercentage<Addr>> = Vec::new();
    for (percentage, placement) in tournament
        .extension
        .distribution
        .iter()
        .zip(placements.iter())
    {
        member_percentages.push(MemberPercentage {
            addr: placement.clone(),
            percentage: *percentage,
        });
    }

    // Set remainder address to first place
    let remainder_addr = placements.first().unwrap().to_owned();

    let distribution = Distribution {
        member_percentages,
        remainder_addr,
    };

    Ok(CompetitionModule::default().inner_process(deps, tournament, Some(distribution))?)
}
