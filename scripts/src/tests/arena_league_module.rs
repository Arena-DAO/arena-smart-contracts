use arena_interface::competition::msg::{
    EscrowInstantiateInfo, ExecuteBaseFns as _, QueryBaseFns as _,
};
use arena_interface::escrow::{ExecuteMsgFns as _, QueryMsgFns as _};
use arena_league_module::msg::{
    ExecuteExtFns as _, LeagueInstantiateExt, LeagueQueryExtFns as _, MatchResultMsg,
};
use arena_league_module::state::{MatchResult, PointAdjustment};
use cosmwasm_std::{coins, to_json_binary, Addr, Coin, Decimal, Int128, Uint128, Uint64};
use cw_balance::{BalanceUnchecked, BalanceVerified, MemberBalanceUnchecked};
use cw_orch::{anyhow, prelude::*};
use cw_utils::Expiration;

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
