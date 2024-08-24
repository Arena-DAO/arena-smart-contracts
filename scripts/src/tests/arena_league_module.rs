use std::str::FromStr;

use arena_interface::competition::msg::{
    EscrowInstantiateInfo, ExecuteBaseFns as _, QueryBaseFns as _,
};
use arena_interface::escrow::{ExecuteMsgFns as _, QueryMsgFns as _};
use arena_league_module::msg::{
    ExecuteExtFns as _, LeagueInstantiateExt, LeagueQueryExtFns as _, MatchResultMsg,
};
use arena_league_module::state::{MatchResult, PointAdjustment};
use cosmwasm_std::{
    coins, to_json_binary, Addr, Coin, CosmosMsg, Decimal, Int128, Uint128, Uint64, WasmMsg,
};
use cw_balance::{BalanceUnchecked, BalanceVerified, MemberBalanceUnchecked};
use cw_orch::{anyhow, prelude::*};
use cw_utils::Expiration;
use dao_proposal_sudo::msg::ExecuteMsgFns;

use crate::tests::helpers::setup_arena;

use super::{DENOM, PREFIX};

#[test]
fn test_create_league() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let teams: Vec<_> = (0..4)
        .map(|i| mock.addr_make_with_balance(format!("team{}", i), coins(10000, DENOM)))
        .collect::<Result<_, _>>()?;

    arena.arena_league_module.set_sender(&admin);

    // Create a league
    let res = arena.arena_league_module.create_competition(
        "A test league".to_string(),
        Expiration::AtHeight(1000000),
        LeagueInstantiateExt {
            teams: teams.iter().map(|team| team.to_string()).collect(),
            match_win_points: Uint64::new(3),
            match_draw_points: Uint64::new(1),
            match_lose_points: Uint64::zero(),
            distribution: vec![
                Decimal::percent(50),
                Decimal::percent(30),
                Decimal::percent(20),
            ],
        },
        "Test League".to_string(),
        None,
        Some(Uint128::one()),
        Some(EscrowInstantiateInfo {
            code_id: arena.arena_escrow.code_id()?,
            msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                dues: teams
                    .iter()
                    .map(|team| MemberBalanceUnchecked {
                        addr: team.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    })
                    .collect(),
                should_activate_on_funded: None,
            })?,
            label: "League Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["League Rule".to_string()]),
        None,
        None,
    )?;

    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "create_competition")));

    // Query the created league
    let league = arena.arena_league_module.competition(Uint128::one())?;
    assert_eq!(league.name, "Test League");

    Ok(())
}

#[test]
fn test_process_league_matches() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let teams: Vec<_> = (0..4)
        .map(|i| mock.addr_make_with_balance(format!("team{}", i), coins(10000, DENOM)))
        .collect::<Result<_, _>>()?;

    arena.arena_league_module.set_sender(&admin);

    // Create a league
    let res = arena.arena_league_module.create_competition(
        "A test league".to_string(),
        Expiration::AtHeight(1000000),
        LeagueInstantiateExt {
            teams: teams.iter().map(|team| team.to_string()).collect(),
            match_win_points: Uint64::new(3),
            match_draw_points: Uint64::new(1),
            match_lose_points: Uint64::zero(),
            distribution: vec![
                Decimal::percent(50),
                Decimal::percent(30),
                Decimal::percent(20),
            ],
        },
        "Test League".to_string(),
        None,
        Some(Uint128::one()),
        Some(EscrowInstantiateInfo {
            code_id: arena.arena_escrow.code_id()?,
            msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                dues: teams
                    .iter()
                    .map(|team| MemberBalanceUnchecked {
                        addr: team.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    })
                    .collect(),
                should_activate_on_funded: None,
            })?,
            label: "League Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["League Rule".to_string()]),
        None,
        None,
    )?;

    let escrow_addr = res
        .events
        .iter()
        .find_map(|event| {
            event
                .attributes
                .iter()
                .find(|attr| attr.key == "escrow_addr")
                .map(|attr| attr.value.clone())
        })
        .ok_or_else(|| anyhow::anyhow!("Escrow address not found in events"))?;

    arena
        .arena_escrow
        .set_address(&Addr::unchecked(escrow_addr));

    // Fund the escrow
    for team in &teams {
        arena.arena_escrow.set_sender(team);
        arena.arena_escrow.receive_native(&coins(1000, DENOM))?;
    }

    // Process matches
    arena.arena_league_module.set_sender(&admin);

    // Process all rounds
    let match_results = vec![
        (
            Uint64::one(),
            vec![
                (Uint128::new(1), MatchResult::Team1),
                (Uint128::new(2), MatchResult::Draw),
            ],
        ),
        (
            Uint64::new(2),
            vec![
                (Uint128::new(3), MatchResult::Team1),
                (Uint128::new(4), MatchResult::Team1),
            ],
        ),
        (
            Uint64::new(3),
            vec![
                (Uint128::new(5), MatchResult::Draw),
                (Uint128::new(6), MatchResult::Team2),
            ],
        ),
    ];

    for (round, results) in match_results {
        arena.arena_league_module.process_match(
            Uint128::one(),
            results
                .into_iter()
                .map(|(number, result)| MatchResultMsg {
                    match_number: number,
                    match_result: result,
                })
                .collect(),
            round,
        )?;
    }

    // Query leaderboard
    let leaderboard = arena
        .arena_league_module
        .leaderboard(Uint128::one(), None)?;

    // Check standings
    assert_eq!(leaderboard[0].points, Int128::new(7)); // 2 wins (3 points each) and 1 draw (1 point)
    assert_eq!(leaderboard[1].points, Int128::new(6)); // 2 wins (3 points each)
    assert_eq!(leaderboard[2].points, Int128::new(2)); // 2 draw (1 point each)
    assert_eq!(leaderboard[3].points, Int128::new(1)); // 1 draw (1 point)

    // Check final balances in the escrow
    let total_prize = Uint128::new(4000); // 1000 stake per team * 4 teams
    let after_tax = total_prize * Decimal::percent(95); // 5% tax

    let expected_balances = [
        Some(BalanceVerified {
            native: Some(coins((after_tax * Decimal::percent(50)).u128(), DENOM)),
            cw20: None,
            cw721: None,
        }),
        Some(BalanceVerified {
            native: Some(coins((after_tax * Decimal::percent(30)).u128(), DENOM)),
            cw20: None,
            cw721: None,
        }),
        Some(BalanceVerified {
            native: Some(coins((after_tax * Decimal::percent(20)).u128(), DENOM)),
            cw20: None,
            cw721: None,
        }),
        None,
    ];

    for (i, member_points) in leaderboard.iter().enumerate() {
        let balance = arena
            .arena_escrow
            .balance(member_points.member.to_string())?;
        assert_eq!(balance, expected_balances[i], "Mismatch for team {}", i);
    }

    Ok(())
}

#[test]
fn test_add_point_adjustments() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let teams: Vec<_> = (0..4)
        .map(|i| mock.addr_make_with_balance(format!("team{}", i), coins(10000, DENOM)))
        .collect::<Result<_, _>>()?;

    arena.arena_league_module.set_sender(&admin);

    // Create a league
    let res = arena.arena_league_module.create_competition(
        "A test league".to_string(),
        Expiration::AtHeight(1000000),
        LeagueInstantiateExt {
            teams: teams.iter().map(|team| team.to_string()).collect(),
            match_win_points: Uint64::new(3),
            match_draw_points: Uint64::new(1),
            match_lose_points: Uint64::zero(),
            distribution: vec![
                Decimal::percent(50),
                Decimal::percent(30),
                Decimal::percent(20),
            ],
        },
        "Test League".to_string(),
        None,
        Some(Uint128::one()),
        Some(EscrowInstantiateInfo {
            code_id: arena.arena_escrow.code_id()?,
            msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                dues: teams
                    .iter()
                    .map(|team| MemberBalanceUnchecked {
                        addr: team.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    })
                    .collect(),
                should_activate_on_funded: None,
            })?,
            label: "League Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["League Rule".to_string()]),
        None,
        None,
    )?;

    let escrow_addr = res
        .events
        .iter()
        .find_map(|event| {
            event
                .attributes
                .iter()
                .find(|attr| attr.key == "escrow_addr")
                .map(|attr| attr.value.clone())
        })
        .ok_or_else(|| anyhow::anyhow!("Escrow address not found in events"))?;

    arena
        .arena_escrow
        .set_address(&Addr::unchecked(escrow_addr));

    // Fund the escrow
    for team in &teams {
        arena.arena_escrow.set_sender(team);
        arena.arena_escrow.receive_native(&coins(1000, DENOM))?;
    }

    // Process a match
    arena.arena_league_module.set_sender(&admin);
    arena.arena_league_module.process_match(
        Uint128::one(),
        vec![MatchResultMsg {
            match_number: Uint128::new(1),
            match_result: MatchResult::Team1,
        }],
        Uint64::one(),
    )?;

    // Add point adjustment
    arena.arena_league_module.add_point_adjustments(
        teams[0].to_string(),
        Uint128::one(),
        vec![PointAdjustment {
            description: "Penalty".to_string(),
            amount: Int128::new(-2),
        }],
    )?;

    // Query leaderboard
    let leaderboard = arena
        .arena_league_module
        .leaderboard(Uint128::one(), None)?;

    // Check standings
    assert_eq!(leaderboard[0].points, Int128::new(1)); // 3 points for win, -2 points for penalty

    Ok(())
}

#[test]
fn test_create_league_with_odd_number_of_teams() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let teams: Vec<_> = (0..5)
        .map(|i| mock.addr_make_with_balance(format!("team{}", i), coins(10000, DENOM)))
        .collect::<Result<_, _>>()?;

    arena.arena_league_module.set_sender(&admin);

    let res = arena.arena_league_module.create_competition(
        "Odd number league",
        Expiration::AtHeight(1000000),
        LeagueInstantiateExt {
            teams: teams.iter().map(|team| team.to_string()).collect(),
            match_win_points: Uint64::new(3),
            match_draw_points: Uint64::new(1),
            match_lose_points: Uint64::zero(),
            distribution: vec![
                Decimal::percent(50),
                Decimal::percent(30),
                Decimal::percent(20),
            ],
        },
        "Odd League",
        None,
        Some(Uint128::one()),
        Some(EscrowInstantiateInfo {
            code_id: arena.arena_escrow.code_id()?,
            msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                dues: teams
                    .iter()
                    .map(|team| MemberBalanceUnchecked {
                        addr: team.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    })
                    .collect(),
                should_activate_on_funded: None,
            })?,
            label: "Odd League Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Odd League Rule".to_string()]),
        None,
        None,
    )?;

    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "create_competition")));

    let league = arena.arena_league_module.competition(Uint128::one())?;
    assert_eq!(league.name, "Odd League");
    assert_eq!(league.extension.teams, Uint64::new(5));

    // Check that the correct number of rounds and matches were created
    let league = arena.arena_league_module.competition(Uint128::one())?;
    assert_eq!(league.extension.rounds, Uint64::new(5)); // n rounds for n teams when n is odd

    Ok(())
}

#[test]
fn test_process_league_with_ties() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let teams: Vec<_> = (0..4)
        .map(|i| mock.addr_make_with_balance(format!("team{}", i), coins(10000, DENOM)))
        .collect::<Result<_, _>>()?;

    arena.arena_league_module.set_sender(&admin);

    let res = arena.arena_league_module.create_competition(
        "Tie league",
        Expiration::AtHeight(1000000),
        LeagueInstantiateExt {
            teams: teams.iter().map(|team| team.to_string()).collect(),
            match_win_points: Uint64::new(3),
            match_draw_points: Uint64::new(1),
            match_lose_points: Uint64::zero(),
            distribution: vec![
                Decimal::percent(40),
                Decimal::percent(30),
                Decimal::percent(20),
                Decimal::percent(10),
            ],
        },
        "Tie League",
        None,
        Some(Uint128::one()),
        Some(EscrowInstantiateInfo {
            code_id: arena.arena_escrow.code_id()?,
            msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                dues: teams
                    .iter()
                    .map(|team| MemberBalanceUnchecked {
                        addr: team.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    })
                    .collect(),
                should_activate_on_funded: None,
            })?,
            label: "Tie League Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Tie League Rule".to_string()]),
        None,
        None,
    )?;

    let escrow_addr = res
        .events
        .iter()
        .find_map(|event| {
            event
                .attributes
                .iter()
                .find(|attr| attr.key == "escrow_addr")
                .map(|attr| attr.value.clone())
        })
        .unwrap();

    arena
        .arena_escrow
        .set_address(&Addr::unchecked(escrow_addr));

    // Fund the escrow
    for team in &teams {
        arena.arena_escrow.set_sender(team);
        arena.arena_escrow.receive_native(&coins(1000, DENOM))?;
    }

    // Process matches with ties
    arena.arena_league_module.set_sender(&admin);

    // Process all rounds
    arena.arena_league_module.process_match(
        Uint128::one(),
        vec![
            MatchResultMsg {
                match_number: Uint128::new(1),
                match_result: MatchResult::Draw,
            },
            MatchResultMsg {
                match_number: Uint128::new(2),
                match_result: MatchResult::Draw,
            },
        ],
        Uint64::one(),
    )?;

    arena.arena_league_module.process_match(
        Uint128::one(),
        vec![
            MatchResultMsg {
                match_number: Uint128::new(3),
                match_result: MatchResult::Draw,
            },
            MatchResultMsg {
                match_number: Uint128::new(4),
                match_result: MatchResult::Draw,
            },
        ],
        Uint64::new(2),
    )?;

    arena.arena_league_module.process_match(
        Uint128::one(),
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
        Uint64::new(3),
    )?;

    let leaderboard = arena
        .arena_league_module
        .leaderboard(Uint128::one(), None)?;

    // Check standings
    assert_eq!(leaderboard[0].points, Int128::new(5)); // 2 draws, 1 win
    assert_eq!(leaderboard[1].points, Int128::new(5)); // 2 draws, 1 win
    assert_eq!(leaderboard[2].points, Int128::new(2)); // 2 draws, 1 loss
    assert_eq!(leaderboard[3].points, Int128::new(2)); // 2 draws, 1 loss

    // Check final balances in the escrow
    let total_prize = Uint128::new(4000); // 1000 stake per team * 4 teams
    let after_tax = total_prize * Decimal::percent(95); // 5% tax

    let expected_balances = [
        Some(BalanceVerified {
            native: Some(coins(
                (after_tax * Decimal::from_str("0.275")?).u128(),
                DENOM,
            )), // (40% + 15%) / 2
            cw20: None,
            cw721: None,
        }),
        Some(BalanceVerified {
            native: Some(coins(
                (after_tax * Decimal::from_str("0.275")?).u128(),
                DENOM,
            )), // (40% + 15%) / 2
            cw20: None,
            cw721: None,
        }),
        Some(BalanceVerified {
            native: Some(coins(
                (after_tax * Decimal::from_str("0.225")?).u128(),
                DENOM,
            )), // (30% + 15%) / 2
            cw20: None,
            cw721: None,
        }),
        Some(BalanceVerified {
            native: Some(coins(
                (after_tax * Decimal::from_str("0.225")?).u128(),
                DENOM,
            )), // (30% + 15%) / 2
            cw20: None,
            cw721: None,
        }),
    ];

    for (i, member_points) in leaderboard.iter().enumerate() {
        let balance = arena
            .arena_escrow
            .balance(member_points.member.to_string())?;
        assert_eq!(balance, expected_balances[i], "Mismatch for team {}", i);
    }

    Ok(())
}

#[test]
fn test_update_distribution() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let teams: Vec<_> = (0..4)
        .map(|i| mock.addr_make_with_balance(format!("team{}", i), coins(10000, DENOM)))
        .collect::<Result<_, _>>()?;

    arena.arena_league_module.set_sender(&admin);

    arena.arena_league_module.create_competition(
        "Distribution league",
        Expiration::AtHeight(1000000),
        LeagueInstantiateExt {
            teams: teams.iter().map(|team| team.to_string()).collect(),
            match_win_points: Uint64::new(3),
            match_draw_points: Uint64::new(1),
            match_lose_points: Uint64::zero(),
            distribution: vec![
                Decimal::percent(40),
                Decimal::percent(30),
                Decimal::percent(20),
                Decimal::percent(10),
            ],
        },
        "Distribution League",
        None,
        Some(Uint128::one()),
        Some(EscrowInstantiateInfo {
            code_id: arena.arena_escrow.code_id()?,
            msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                dues: teams
                    .iter()
                    .map(|team| MemberBalanceUnchecked {
                        addr: team.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    })
                    .collect(),
                should_activate_on_funded: None,
            })?,
            label: "Distribution League Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Distribution League Rule".to_string()]),
        None,
        None,
    )?;

    // Update distribution
    let new_distribution = vec![
        Decimal::percent(50),
        Decimal::percent(25),
        Decimal::percent(15),
        Decimal::percent(10),
    ];

    arena
        .dao_dao
        .dao_proposal_sudo
        .call_as(&admin)
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.arena_league_module.addr_str()?,
            msg: to_json_binary(&arena_league_module::msg::ExecuteMsg::Extension {
                msg: arena_league_module::msg::ExecuteExt::UpdateDistribution {
                    league_id: Uint128::one(),
                    distribution: new_distribution.clone(),
                },
            })?,
            funds: vec![],
        })])?;
    mock.next_block()?;

    // Verify the updated distribution
    let updated_league = arena.arena_league_module.competition(Uint128::one())?;
    assert_eq!(updated_league.extension.distribution, new_distribution);

    Ok(())
}

#[test]
fn test_create_league_with_invalid_team_count() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    arena.arena_league_module.set_sender(&admin);

    // Attempt to create a league with only one team
    let result = arena.arena_league_module.create_competition(
        "Invalid league",
        Expiration::AtHeight(1000000),
        LeagueInstantiateExt {
            teams: vec!["team1".to_string()],
            match_win_points: Uint64::new(3),
            match_draw_points: Uint64::new(1),
            match_lose_points: Uint64::zero(),
            distribution: vec![Decimal::percent(100)],
        },
        "Invalid League",
        None,
        Some(Uint128::one()),
        None,
        None,
        Some(vec!["Invalid League Rule".to_string()]),
        None,
        None,
    );

    assert!(result.is_err());

    // Attempt to create a league with too many teams (e.g., 101)
    let many_teams: Vec<String> = (1..=101).map(|i| format!("team{}", i)).collect();
    let result = arena.arena_league_module.create_competition(
        "Too Many Teams League",
        Expiration::AtHeight(1000000),
        LeagueInstantiateExt {
            teams: many_teams,
            match_win_points: Uint64::new(3),
            match_draw_points: Uint64::new(1),
            match_lose_points: Uint64::zero(),
            distribution: vec![Decimal::percent(100)],
        },
        "Too Many Teams League",
        None,
        Some(Uint128::one()),
        None,
        None,
        Some(vec!["Too Many Teams League Rule".to_string()]),
        None,
        None,
    );

    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_process_matches_out_of_order() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let teams: Vec<_> = (0..4)
        .map(|i| mock.addr_make_with_balance(format!("team{}", i), coins(10000, DENOM)))
        .collect::<Result<_, _>>()?;

    arena.arena_league_module.set_sender(&admin);

    arena.arena_league_module.create_competition(
        "Out of Order League",
        Expiration::AtHeight(1000000),
        LeagueInstantiateExt {
            teams: teams.iter().map(|team| team.to_string()).collect(),
            match_win_points: Uint64::new(3),
            match_draw_points: Uint64::new(1),
            match_lose_points: Uint64::zero(),
            distribution: vec![
                Decimal::percent(40),
                Decimal::percent(30),
                Decimal::percent(20),
                Decimal::percent(10),
            ],
        },
        "Out of Order League",
        None,
        Some(Uint128::one()),
        Some(EscrowInstantiateInfo {
            code_id: arena.arena_escrow.code_id()?,
            msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                dues: teams
                    .iter()
                    .map(|team| MemberBalanceUnchecked {
                        addr: team.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    })
                    .collect(),
                should_activate_on_funded: None,
            })?,
            label: "Out of Order League Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Out of Order League Rule".to_string()]),
        None,
        None,
    )?;

    // Attempt to process matches from round 2 before round 1
    let result = arena.arena_league_module.process_match(
        Uint128::one(),
        vec![
            MatchResultMsg {
                match_number: Uint128::new(3),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(4),
                match_result: MatchResult::Team2,
            },
        ],
        Uint64::new(2),
    );

    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_multiple_point_adjustments() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let teams: Vec<_> = (0..4)
        .map(|i| mock.addr_make_with_balance(format!("team{}", i), coins(10000, DENOM)))
        .collect::<Result<_, _>>()?;

    arena.arena_league_module.set_sender(&admin);

    let res = arena.arena_league_module.create_competition(
        "Multiple Adjustments League",
        Expiration::AtHeight(1000000),
        LeagueInstantiateExt {
            teams: teams.iter().map(|team| team.to_string()).collect(),
            match_win_points: Uint64::new(3),
            match_draw_points: Uint64::new(1),
            match_lose_points: Uint64::zero(),
            distribution: vec![
                Decimal::percent(40),
                Decimal::percent(30),
                Decimal::percent(20),
                Decimal::percent(10),
            ],
        },
        "Multiple Adjustments League",
        None,
        Some(Uint128::one()),
        Some(EscrowInstantiateInfo {
            code_id: arena.arena_escrow.code_id()?,
            msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                dues: teams
                    .iter()
                    .map(|team| MemberBalanceUnchecked {
                        addr: team.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    })
                    .collect(),
                should_activate_on_funded: None,
            })?,
            label: "Multiple Adjustments League Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Multiple Adjustments League Rule".to_string()]),
        None,
        None,
    )?;

    let escrow_addr = res
        .events
        .iter()
        .find_map(|event| {
            event
                .attributes
                .iter()
                .find(|attr| attr.key == "escrow_addr")
                .map(|attr| attr.value.clone())
        })
        .ok_or_else(|| anyhow::anyhow!("Escrow address not found in events"))?;

    arena
        .arena_escrow
        .set_address(&Addr::unchecked(escrow_addr));

    // Fund the escrow
    for team in &teams {
        arena.arena_escrow.set_sender(team);
        arena.arena_escrow.receive_native(&coins(1000, DENOM))?;
    }

    // Process some matches
    arena.arena_league_module.process_match(
        Uint128::one(),
        vec![
            MatchResultMsg {
                match_number: Uint128::new(1),
                match_result: MatchResult::Team1,
            },
            MatchResultMsg {
                match_number: Uint128::new(2),
                match_result: MatchResult::Team2,
            },
        ],
        Uint64::one(),
    )?;

    // Add multiple point adjustments
    arena.arena_league_module.add_point_adjustments(
        teams[0].to_string(),
        Uint128::one(),
        vec![
            PointAdjustment {
                description: "Penalty 1".to_string(),
                amount: Int128::new(-1),
            },
            PointAdjustment {
                description: "Bonus".to_string(),
                amount: Int128::new(2),
            },
        ],
    )?;

    arena.arena_league_module.add_point_adjustments(
        teams[0].to_string(),
        Uint128::one(),
        vec![PointAdjustment {
            description: "Penalty 2".to_string(),
            amount: Int128::new(-1),
        }],
    )?;

    let leaderboard = arena
        .arena_league_module
        .leaderboard(Uint128::one(), None)?;

    // Check standings
    assert_eq!(leaderboard[0].points, Int128::new(3)); // 3 points for win
    assert_eq!(leaderboard[1].points, Int128::new(3)); // 3 points for win - 1 + 2 - 1 = 3
    assert_eq!(leaderboard[2].points, Int128::new(0));
    assert_eq!(leaderboard[3].points, Int128::new(0));

    Ok(())
}
