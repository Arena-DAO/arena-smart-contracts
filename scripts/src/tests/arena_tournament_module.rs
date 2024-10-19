use std::str::FromStr;

use arena_interface::{
    competition::msg::EscrowInstantiateInfo,
    core::QueryExtFns as _,
    escrow::{ExecuteMsgFns as _, QueryMsgFns as _},
    group::{self, AddMemberMsg, GroupContractInfo},
};
use arena_tournament_module::{
    msg::{
        ExecuteExtFns as _, ExecuteMsg, MatchResultMsg, MigrateMsg, QueryExtFns as _,
        TournamentInstantiateExt,
    },
    state::{EliminationType, MatchResult},
};
use cosmwasm_std::{coins, to_json_binary, Decimal, Uint128};
use cw_balance::{BalanceUnchecked, MemberBalanceUnchecked};
use cw_orch::{anyhow, environment::ChainState, prelude::*};
use cw_orch_clone_testing::CloneTesting;
use dao_interface::state::ModuleInstantiateInfo;
use itertools::Itertools;
use networks::PION_1;

use crate::{
    tests::helpers::{setup_arena, setup_voting_module},
    Arena,
};

use super::{DENOM, PREFIX};

#[test]
pub fn test_tournament_instantiate() -> Result<(), CwOrchError> {
    let mock = MockBech32::new(PREFIX);
    let (arena, _admin) = setup_arena(&mock)?;

    // Set teams
    let mut teams = vec![];
    for i in 0..10 {
        teams.push(mock.addr_make_with_balance(format!("team {}", i), coins(100_000u128, DENOM))?);
    }

    // Larger distribution than possible placements
    let result = arena.arena_tournament_module.execute(
        &create_competition_msg(
            &arena,
            None,
            &teams,
            EliminationType::SingleElimination {
                play_third_place_match: false,
            },
            vec![
                Decimal::from_ratio(75u128, 100u128),
                Decimal::from_ratio(15u128, 100u128),
                Decimal::from_ratio(10u128, 100u128),
            ],
        )?,
        None,
    );
    assert!(result.is_err());

    // Larger distribution than possible placements - 3rd place match
    let result = arena.arena_tournament_module.execute(
        &create_competition_msg(
            &arena,
            None,
            &teams,
            EliminationType::SingleElimination {
                play_third_place_match: true,
            },
            vec![
                Decimal::from_ratio(75u128, 100u128),
                Decimal::from_ratio(10u128, 100u128),
                Decimal::from_ratio(5u128, 100u128),
                Decimal::from_ratio(5u128, 100u128),
                Decimal::from_ratio(5u128, 100u128),
            ],
        )?,
        None,
    );
    assert!(result.is_err());

    // Distribution does not sum to 1
    let result = arena.arena_tournament_module.execute(
        &create_competition_msg(
            &arena,
            None,
            &teams,
            EliminationType::SingleElimination {
                play_third_place_match: false,
            },
            vec![
                Decimal::from_ratio(100u128, 100u128),
                Decimal::from_ratio(100u128, 100u128),
            ],
        )?,
        None,
    );
    assert!(result.is_err());

    // Larger distribution than possible placements - double elim
    let result = arena.arena_tournament_module.execute(
        &create_competition_msg(
            &arena,
            None,
            &teams,
            EliminationType::DoubleElimination {},
            vec![
                Decimal::from_ratio(75u128, 100u128),
                Decimal::from_ratio(10u128, 100u128),
                Decimal::from_ratio(10u128, 100u128),
                Decimal::from_ratio(5u128, 100u128),
            ],
        )?,
        None,
    );
    assert!(result.is_err());

    Ok(())
}

#[test]
pub fn test_single_elimination_tournament() -> Result<(), CwOrchError> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Set teams
    let mut teams = vec![];
    for i in 0..10 {
        teams.push(mock.addr_make_with_balance(format!("team {}", i), coins(100_000u128, DENOM))?);
    }

    arena.arena_tournament_module.set_sender(&admin);

    // Create a tournament w/ 10k due from each team
    let response = arena.arena_tournament_module.execute(
        &create_competition_msg(
            &arena,
            None,
            &teams,
            EliminationType::SingleElimination {
                play_third_place_match: false,
            },
            vec![
                Decimal::from_ratio(75u128, 100u128),
                Decimal::from_ratio(25u128, 100u128),
            ],
        )?,
        None,
    )?;
    mock.next_block()?;

    // Get and set escrow addr
    let escrow_addr = response.events.iter().find_map(|event| {
        event
            .attributes
            .iter()
            .find(|attr| attr.key == "escrow_addr")
            .map(|attr| attr.value.clone())
    });
    assert!(escrow_addr.is_some());
    arena
        .arena_escrow
        .set_address(&Addr::unchecked(escrow_addr.unwrap()));

    // Processing without funding is an error
    let result = arena.arena_tournament_module.process_match(
        vec![MatchResultMsg {
            match_number: Uint128::one(),
            match_result: MatchResult::Team1,
        }],
        Uint128::one(),
    );
    assert!(result.is_err());

    // Fund tournament
    for team in teams.iter() {
        arena.arena_escrow.set_sender(team);
        arena
            .arena_escrow
            .receive_native(&coins(10_000u128, DENOM))?;
    }

    // This should create 9 matches
    let bracket = arena
        .arena_tournament_module
        .bracket(Uint128::one(), None)?;
    assert_eq!(bracket.len(), 9);

    // Attempting to process an unpopulated match errors
    let result = arena.arena_tournament_module.process_match(
        vec![MatchResultMsg {
            match_number: Uint128::new(9), // The final
            match_result: MatchResult::Team1,
        }],
        Uint128::one(),
    );
    assert!(result.is_err());

    // Process first round of matches
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(1),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(2),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(4),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(6),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Process 2nd round of matches
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(3),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(5),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Process 3rd round of matches
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(7),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(8),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Process the final match
    arena.arena_tournament_module.process_match(
        vec![MatchResultMsg {
            match_number: Uint128::new(9),
            match_result: MatchResult::Team1,
        }],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Check distribution
    let balances = arena.arena_escrow.balances(None, None)?;
    assert_eq!(balances.len(), 2);
    assert_eq!(
        balances[1].balance.native.as_ref().unwrap()[0].amount,
        Uint128::new(23750) // 100k * .95 (Arena tax) * .25 (user share)
    );
    assert_eq!(
        balances[0].balance.native.as_ref().unwrap()[0].amount,
        Uint128::new(71250) // 100k * .95 (Arena tax) * .75 (user share)
    );

    Ok(())
}

#[test]
pub fn test_ratings() -> Result<(), CwOrchError> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;
    setup_voting_module(
        &mock,
        &arena,
        vec![cw4::Member {
            addr: admin.to_string(),
            weight: 1u64,
        }],
    )?;

    // Set teams
    let mut teams = vec![];
    for i in 0..10 {
        teams.push(mock.addr_make_with_balance(format!("team {}", i), coins(100_000u128, DENOM))?);
    }

    arena.arena_tournament_module.set_sender(&admin);

    // Create a tournament on category 1
    let response = arena.arena_tournament_module.execute(
        &create_competition_msg(
            &arena,
            Some(Uint128::one()),
            &teams,
            EliminationType::SingleElimination {
                play_third_place_match: false,
            },
            vec![
                Decimal::from_ratio(75u128, 100u128),
                Decimal::from_ratio(25u128, 100u128),
            ],
        )?,
        None,
    )?;
    mock.next_block()?;

    // Get and set escrow addr
    let escrow_addr = response.events.iter().find_map(|event| {
        event
            .attributes
            .iter()
            .find(|attr| attr.key == "escrow_addr")
            .map(|attr| attr.value.clone())
    });
    assert!(escrow_addr.is_some());
    arena
        .arena_escrow
        .set_address(&Addr::unchecked(escrow_addr.unwrap()));

    // Fund tournament
    for team in teams.iter() {
        arena.arena_escrow.set_sender(team);
        arena
            .arena_escrow
            .receive_native(&coins(10_000u128, DENOM))?;
    }

    // Process first round of matches
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(1),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(2),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(4),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(6),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Check ratings after first round
    let ratings_after_first_round =
        arena
            .arena_core
            .rating_leaderboard(Uint128::one(), None, None)?;
    assert_eq!(ratings_after_first_round.len(), 8);

    // Process 2nd round of matches
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(3),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(5),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Check ratings after second round
    let ratings_after_second_round =
        arena
            .arena_core
            .rating_leaderboard(Uint128::one(), None, None)?;
    assert_eq!(ratings_after_second_round.len(), 10);

    // Process 3rd round of matches
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(7),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(8),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Process the final match
    arena.arena_tournament_module.process_match(
        vec![MatchResultMsg {
            match_number: Uint128::new(9),
            match_result: MatchResult::Team1,
        }],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Check final ratings
    let final_ratings = arena
        .arena_core
        .rating_leaderboard(Uint128::one(), None, None)?;
    assert_eq!(final_ratings.len(), 10);

    // Verify that ratings have changed
    assert_ne!(ratings_after_first_round, final_ratings);

    // Verify winner has the highest rating
    let winner_rating = final_ratings[0].rating.clone();
    assert!(winner_rating.value > Decimal::from_str("1500")?);

    // Verify loser has the lowest rating
    let loser_rating = final_ratings.last().unwrap().rating.clone();
    assert!(loser_rating.value < Decimal::from_str("1500")?);

    Ok(())
}

#[test]
pub fn test_single_elimination_tournament_with_third_place_match() -> Result<(), CwOrchError> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Set teams
    let mut teams = vec![];
    for i in 0..10 {
        teams.push(mock.addr_make_with_balance(format!("team {}", i), coins(100_000u128, DENOM))?);
    }

    arena.arena_tournament_module.set_sender(&admin);

    // Create a tournament w/ 10k due from each team
    let response = arena.arena_tournament_module.execute(
        &create_competition_msg(
            &arena,
            None,
            &teams,
            EliminationType::SingleElimination {
                play_third_place_match: true,
            },
            vec![
                Decimal::from_ratio(65u128, 100u128),
                Decimal::from_ratio(15u128, 100u128),
                Decimal::from_ratio(10u128, 100u128),
                Decimal::from_ratio(10u128, 100u128),
            ],
        )?,
        None,
    )?;
    mock.next_block()?;

    // Get and set escrow addr
    let escrow_addr = response.events.iter().find_map(|event| {
        event
            .attributes
            .iter()
            .find(|attr| attr.key == "escrow_addr")
            .map(|attr| attr.value.clone())
    });
    assert!(escrow_addr.is_some());
    arena
        .arena_escrow
        .set_address(&Addr::unchecked(escrow_addr.unwrap()));

    // Fund tournament
    for team in teams.iter() {
        arena.arena_escrow.set_sender(team);
        arena
            .arena_escrow
            .receive_native(&coins(10_000u128, DENOM))?;
    }

    // This should create 10 matches
    let bracket = arena
        .arena_tournament_module
        .bracket(Uint128::new(1u128), None)?;
    assert_eq!(bracket.len(), 10);

    // Process first round of matches
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(1),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(2),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(4),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(6),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Process 2nd round of matches
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(3),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(5),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Process 3rd round of matches
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(7),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(8),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Process the final matches
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(9),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(10),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Check distribution
    let balances = arena.arena_escrow.balances(None, None)?;
    assert_eq!(balances.len(), 4);
    assert_eq!(
        balances[0].balance.native.as_ref().unwrap()[0].amount,
        Uint128::new(61750) // 100k * .95 (Arena tax) * .65 (user share)
    );
    assert_eq!(
        balances[1].balance.native.as_ref().unwrap()[0].amount,
        Uint128::new(9500) // 100k * .95 (Arena tax) * .10 (user share)
    );
    assert_eq!(
        balances[2].balance.native.as_ref().unwrap()[0].amount,
        Uint128::new(14250) // 100k * .95 (Arena tax) * .15 (user share)
    );
    assert_eq!(
        balances[3].balance.native.as_ref().unwrap()[0].amount,
        Uint128::new(9500) // 100k * .95 (Arena tax) * .10 (user share)
    );

    Ok(())
}

#[test]
pub fn test_double_elimination_tournament_with_rebuttal() -> Result<(), CwOrchError> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Set teams
    let mut teams = vec![];
    for i in 0..10 {
        teams.push(mock.addr_make_with_balance(format!("team {}", i), coins(100_000u128, DENOM))?);
    }
    arena.arena_tournament_module.set_sender(&admin);

    // Create a tournament w/ 10k due from each team
    let response = arena.arena_tournament_module.execute(
        &create_competition_msg(
            &arena,
            None,
            &teams,
            EliminationType::DoubleElimination {},
            vec![
                Decimal::from_ratio(65u128, 100u128),
                Decimal::from_ratio(25u128, 100u128),
                Decimal::from_ratio(10u128, 100u128),
            ],
        )?,
        None,
    )?;
    mock.next_block()?;

    // Get and set escrow addr
    let escrow_addr = response.events.iter().find_map(|event| {
        event
            .attributes
            .iter()
            .find(|attr| attr.key == "escrow_addr")
            .map(|attr| attr.value.clone())
    });
    assert!(escrow_addr.is_some());
    arena
        .arena_escrow
        .set_address(&Addr::unchecked(escrow_addr.unwrap()));

    // Fund tournament
    for team in teams.iter() {
        arena.arena_escrow.set_sender(team);
        arena
            .arena_escrow
            .receive_native(&coins(10_000u128, DENOM))?;
    }

    // This should create 2(n-1) matches
    let bracket = arena
        .arena_tournament_module
        .bracket(Uint128::new(1u128), None)?;
    assert_eq!(bracket.len(), 18);

    // Process first round of matches
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(2),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(4),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(5),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(6),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Process 2nd round of matches
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(1),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(3),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Process base round of loser's bracket
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(7),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(8),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Process 1st round of loser's bracket
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(11),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(12),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // 2nd round of winner's bracket
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(9),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(10),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // 2nd round of loser's bracket
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(13),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(14),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Last round of loser's bracket before final
    arena.arena_tournament_module.process_match(
        vec![MatchResultMsg {
            match_number: Uint128::new(15),
            match_result: MatchResult::Team1,
        }],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Winner's bracket final
    arena.arena_tournament_module.process_match(
        vec![MatchResultMsg {
            match_number: Uint128::new(16),
            match_result: MatchResult::Team1,
        }],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Loser's bracket final
    arena.arena_tournament_module.process_match(
        vec![MatchResultMsg {
            match_number: Uint128::new(17),
            match_result: MatchResult::Team1,
        }],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Grand final
    arena.arena_tournament_module.process_match(
        vec![MatchResultMsg {
            match_number: Uint128::new(18),
            match_result: MatchResult::Team2,
        }],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Rebuttal match
    arena.arena_tournament_module.process_match(
        vec![MatchResultMsg {
            match_number: Uint128::new(19),
            match_result: MatchResult::Team1,
        }],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Check distribution
    let balances = arena.arena_escrow.balances(None, None)?;
    assert_eq!(balances.len(), 3);
    assert_eq!(
        balances[0].balance.native.as_ref().unwrap()[0].amount,
        Uint128::new(23750) // 100k * .95 (Arena tax) * .25 (user share)
    );
    assert_eq!(
        balances[1].balance.native.as_ref().unwrap()[0].amount,
        Uint128::new(61750) // 100k * .95 (Arena tax) * .65 (user share)
    );
    assert_eq!(
        balances[2].balance.native.as_ref().unwrap()[0].amount,
        Uint128::new(9500) // 100k * .95 (Arena tax) * .10 (user share)
    );

    Ok(())
}

#[test]
pub fn test_double_elimination_tournament() -> Result<(), CwOrchError> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Set teams
    let mut teams = vec![];
    for i in 0..10 {
        teams.push(mock.addr_make_with_balance(format!("team {}", i), coins(100_000u128, DENOM))?);
    }
    arena.arena_tournament_module.set_sender(&admin);

    // Create a tournament w/ 10k due from each team
    let response = arena.arena_tournament_module.execute(
        &create_competition_msg(
            &arena,
            None,
            &teams,
            EliminationType::DoubleElimination {},
            vec![
                Decimal::from_ratio(65u128, 100u128),
                Decimal::from_ratio(25u128, 100u128),
                Decimal::from_ratio(10u128, 100u128),
            ],
        )?,
        None,
    )?;
    mock.next_block()?;

    // Get and set escrow addr
    let escrow_addr = response.events.iter().find_map(|event| {
        event
            .attributes
            .iter()
            .find(|attr| attr.key == "escrow_addr")
            .map(|attr| attr.value.clone())
    });
    assert!(escrow_addr.is_some());
    arena
        .arena_escrow
        .set_address(&Addr::unchecked(escrow_addr.unwrap()));

    // Fund tournament
    for team in teams.iter() {
        arena.arena_escrow.set_sender(team);
        arena
            .arena_escrow
            .receive_native(&coins(10_000u128, DENOM))?;
    }

    // This should create 2(n-1) matches
    let bracket = arena
        .arena_tournament_module
        .bracket(Uint128::new(1u128), None)?;
    assert_eq!(bracket.len(), 18);

    // Process first round of matches
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(2),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(4),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(5),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(6),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Process 2nd round of matches
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(1),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(3),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Process base round of loser's bracket
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(7),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(8),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Process 1st round of loser's bracket
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(11),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(12),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // 2nd round of winner's bracket
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(9),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(10),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // 2nd round of loser's bracket
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(13),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(14),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Last round of loser's bracket before final
    arena.arena_tournament_module.process_match(
        vec![MatchResultMsg {
            match_number: Uint128::new(15),
            match_result: MatchResult::Team1,
        }],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Winner's bracket final
    arena.arena_tournament_module.process_match(
        vec![MatchResultMsg {
            match_number: Uint128::new(16),
            match_result: MatchResult::Team1,
        }],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Loser's bracket final
    arena.arena_tournament_module.process_match(
        vec![MatchResultMsg {
            match_number: Uint128::new(17),
            match_result: MatchResult::Team1,
        }],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Grand final
    arena.arena_tournament_module.process_match(
        vec![MatchResultMsg {
            match_number: Uint128::new(18),
            match_result: MatchResult::Team1,
        }],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Check distribution
    let balances = arena.arena_escrow.balances(None, None)?;
    assert_eq!(balances.len(), 3);
    assert_eq!(
        balances[0].balance.native.as_ref().unwrap()[0].amount,
        Uint128::new(23750) // 100k * .95 (Arena tax) * .25 (user share)
    );
    assert_eq!(
        balances[1].balance.native.as_ref().unwrap()[0].amount,
        Uint128::new(61750) // 100k * .95 (Arena tax) * .65 (user share)
    );
    assert_eq!(
        balances[2].balance.native.as_ref().unwrap()[0].amount,
        Uint128::new(9500) // 100k * .95 (Arena tax) * .10 (user share)
    );

    Ok(())
}

// 6 teams is an interesting number, because this has 2 byes leading directly into the semifinals
// It's easier to test seeding here as well
#[test]
pub fn test_single_elimination_6() -> Result<(), CwOrchError> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Set teams
    let mut teams = vec![];
    for i in 0..6 {
        teams.push(mock.addr_make_with_balance(format!("team {}", i), coins(100_000u128, DENOM))?);
    }
    arena.arena_tournament_module.set_sender(&admin);

    // Create a tournament w/ 10k due from each team
    let response = arena.arena_tournament_module.execute(
        &create_competition_msg(
            &arena,
            None,
            &teams,
            EliminationType::SingleElimination {
                play_third_place_match: true,
            },
            vec![
                Decimal::from_ratio(65u128, 100u128),
                Decimal::from_ratio(25u128, 100u128),
                Decimal::from_ratio(10u128, 100u128),
            ],
        )?,
        None,
    )?;
    mock.next_block()?;

    // Get and set escrow addr
    let escrow_addr = response.events.iter().find_map(|event| {
        event
            .attributes
            .iter()
            .find(|attr| attr.key == "escrow_addr")
            .map(|attr| attr.value.clone())
    });
    assert!(escrow_addr.is_some());
    arena
        .arena_escrow
        .set_address(&Addr::unchecked(escrow_addr.unwrap()));

    // Fund tournament
    for team in teams.iter() {
        arena.arena_escrow.set_sender(team);
        arena
            .arena_escrow
            .receive_native(&coins(10_000u128, DENOM))?;
    }

    // This should create n matches
    let bracket = arena
        .arena_tournament_module
        .bracket(Uint128::new(1u128), None)?;
    assert_eq!(bracket.len(), 6);

    // Assert that the bottom 4 teams are in these matches
    let bottom_4 = teams.iter().skip(2).collect_vec();
    assert!(bottom_4
        .iter()
        .any(|x| x == bracket[0].team_1.as_ref().unwrap()));
    assert!(bottom_4
        .iter()
        .any(|x| x == bracket[0].team_2.as_ref().unwrap()));
    assert!(bottom_4
        .iter()
        .any(|x| x == bracket[1].team_1.as_ref().unwrap()));
    assert!(bottom_4
        .iter()
        .any(|x| x == bracket[1].team_2.as_ref().unwrap()));

    // Process the non-byes
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(1),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(2),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Process the semifinals
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(3),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(4),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Process the final matches
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(5),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(6),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Check distribution
    let balances = arena.arena_escrow.balances(None, None)?;
    assert_eq!(balances.len(), 3);
    assert_eq!(
        balances[0].balance.native.as_ref().unwrap()[0].amount,
        Uint128::new(5700) // 60k * .95 (Arena tax) * .10 (user share)
    );
    assert_eq!(
        balances[1].balance.native.as_ref().unwrap()[0].amount,
        Uint128::new(14250) // 60k * .95 (Arena tax) * .25 (user share)
    );
    assert_eq!(
        balances[2].balance.native.as_ref().unwrap()[0].amount,
        Uint128::new(37050) // 60k * .95 (Arena tax) * .65 (user share)
    );

    Ok(())
}

#[test]
pub fn test_double_elimination_many_teams() -> Result<(), CwOrchError> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Set teams
    let mut teams = vec![];
    /*
    2^14 (16384) generation was about 2 seconds
    2^17 (131072) generation was about 24 seconds
    If we want a ton of teams, we can optimize for large quantities of participants by bypassing the nested-seeding algorithm
     */
    for i in 0..10_000 {
        teams.push(mock.addr_make_with_balance(format!("team {}", i), coins(100_000u128, DENOM))?);
    }
    arena.arena_tournament_module.set_sender(&admin);

    // Create a tournament w/ 10k due from each team
    arena.arena_tournament_module.execute(
        &create_competition_msg(
            &arena,
            None,
            &teams,
            EliminationType::DoubleElimination {},
            vec![
                Decimal::from_ratio(65u128, 100u128),
                Decimal::from_ratio(25u128, 100u128),
                Decimal::from_ratio(10u128, 100u128),
            ],
        )?,
        None,
    )?;
    mock.next_block()?;

    Ok(())
}

#[test]
pub fn test_match_updates() -> Result<(), CwOrchError> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    // Set teams
    let mut teams = vec![];
    for i in 0..10 {
        teams.push(mock.addr_make_with_balance(format!("team {}", i), coins(100_000u128, DENOM))?);
    }
    arena.arena_tournament_module.set_sender(&admin);

    // Create a tournament w/ 10k due from each team
    let response = arena.arena_tournament_module.execute(
        &create_competition_msg(
            &arena,
            None,
            &teams,
            EliminationType::DoubleElimination {},
            vec![
                Decimal::from_ratio(65u128, 100u128),
                Decimal::from_ratio(25u128, 100u128),
                Decimal::from_ratio(10u128, 100u128),
            ],
        )?,
        None,
    )?;
    mock.next_block()?;

    // Get and set escrow addr
    let escrow_addr = response.events.iter().find_map(|event| {
        event
            .attributes
            .iter()
            .find(|attr| attr.key == "escrow_addr")
            .map(|attr| attr.value.clone())
    });
    assert!(escrow_addr.is_some());
    arena
        .arena_escrow
        .set_address(&Addr::unchecked(escrow_addr.unwrap()));

    // Fund tournament
    for team in teams.iter() {
        arena.arena_escrow.set_sender(team);
        arena
            .arena_escrow
            .receive_native(&coins(10_000u128, DENOM))?;
    }

    // This should create 2(n-1) matches
    let bracket = arena
        .arena_tournament_module
        .bracket(Uint128::new(1u128), None)?;
    assert_eq!(bracket.len(), 18);

    // Process first round of matches
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(2),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(4),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(5),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(6),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Process 2nd round of matches
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(1),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(3),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Process base round of loser's bracket
    arena.arena_tournament_module.process_match(
        vec![
            MatchResultMsg {
                match_number: Uint128::new(7),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(8),
                match_result: MatchResult::Team1,
            },
        ],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Get match to update
    let match_ = arena
        .arena_tournament_module
        .r#match(Uint128::new(2), Uint128::one())?;
    let previous_winner = match_.team_1;

    // Update a match result
    arena.arena_tournament_module.process_match(
        vec![MatchResultMsg {
            match_number: Uint128::new(2),
            match_result: MatchResult::Team2,
        }],
        Uint128::one(),
    )?;
    mock.next_block()?;

    // Get match again
    let match_ = arena
        .arena_tournament_module
        .r#match(Uint128::new(2), Uint128::one())?;
    assert_eq!(match_.result, Some(MatchResult::Team2 {}));

    // Get next match winner
    let next_match_winner = arena
        .arena_tournament_module
        .r#match(Uint128::new(8), Uint128::one())?;
    assert_ne!(next_match_winner.team_1, previous_winner);
    assert_ne!(next_match_winner.team_2, previous_winner);

    // Get next match loser
    let next_match_loser = arena
        .arena_tournament_module
        .r#match(Uint128::new(9), Uint128::one())?;
    assert_ne!(next_match_loser.team_1, previous_winner);
    assert_ne!(next_match_loser.team_2, previous_winner);

    Ok(())
}

#[test]
fn test_migration_v2_v2_1() -> anyhow::Result<()> {
    let app = CloneTesting::new(PION_1)?;
    let mut arena = Arena::new(app.clone());
    const ARENA_DAO: &str = "neutron1ehkcl0n6s2jtdw75xsvfxm304mz4hs5z7jt6wn5mk0celpj0epqql4ulxk";
    let arena_dao_addr = Addr::unchecked(ARENA_DAO);

    arena.arena_group.upload()?;
    arena.arena_tournament_module.upload()?;

    arena.arena_group.instantiate(
        &group::InstantiateMsg { members: None },
        Some(&arena_dao_addr),
        None,
    )?;

    arena.arena_tournament_module.set_address(&Addr::unchecked(
        "neutron1trp2yfy9meae6nkux7dp8mwkf5ztks3gpkdjqv0xewevqvspktrsgztphp",
    ));
    arena.arena_tournament_module.set_sender(&arena_dao_addr);

    arena.arena_tournament_module.migrate(
        &MigrateMsg::WithGroupAddress {
            group_contract: arena.arena_group.addr_str()?,
        },
        arena.arena_tournament_module.code_id()?,
    )?;

    Ok(())
}

fn create_competition_msg<Chain: ChainState>(
    arena: &Arena<Chain>,
    category_id: Option<Uint128>,
    teams: &[Addr],
    elimination_type: EliminationType,
    distribution: Vec<Decimal>,
) -> Result<ExecuteMsg, CwOrchError> {
    Ok(ExecuteMsg::CreateCompetition {
        category_id,
        host: None,
        escrow: Some(EscrowInstantiateInfo {
            code_id: arena.arena_escrow.code_id().unwrap(),
            msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                dues: teams
                    .iter()
                    .map(|x| MemberBalanceUnchecked {
                        addr: x.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(coins(10_000u128, DENOM)),
                            cw20: None,
                            cw721: None,
                        },
                    })
                    .collect(),
            })
            .unwrap(),
            label: "Arena Escrow".to_string(),
            additional_layered_fees: None,
        }),
        name: "Competition".to_string(),
        description: "Competition description".to_string(),
        expiration: cw_utils::Expiration::Never {},
        rules: None,
        rulesets: None,
        banner: None,
        instantiate_extension: TournamentInstantiateExt {
            elimination_type,
            distribution,
        },
        group_contract: GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: Some(
                        teams
                            .iter()
                            .map(|x| AddMemberMsg {
                                addr: x.to_string(),
                                seed: None,
                            })
                            .collect(),
                    ),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
    })
}
