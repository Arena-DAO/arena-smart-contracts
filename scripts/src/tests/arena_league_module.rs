use std::str::FromStr;

use arena_interface::competition::msg::{
    EscrowInstantiateInfo, ExecuteBaseFns as _, QueryBaseFns as _,
};
use arena_interface::competition::stats::{
    MemberStatsMsg, StatAggregationType, StatMsg, StatType, StatValue, StatValueType,
};
use arena_interface::core::QueryExtFns as _;
use arena_interface::escrow::{ExecuteMsgFns as _, QueryMsgFns as _};
use arena_interface::group::{self, GroupContractInfo};
use arena_league_module::msg::{
    ExecuteExtFns as _, LeagueInstantiateExt, LeagueQueryExtFns as _, MatchResultMsg, MigrateMsg,
};
use arena_league_module::state::{MatchResult, PointAdjustment};
use cosmwasm_std::{
    coins, to_json_binary, Addr, Coin, CosmosMsg, Decimal, Int128, Uint128, Uint64, WasmMsg,
};
use cw_balance::{BalanceUnchecked, BalanceVerified, MemberBalanceUnchecked};
use cw_orch::{anyhow, prelude::*};
use cw_orch_clone_testing::CloneTesting;
use cw_utils::Expiration;
use dao_interface::state::ModuleInstantiateInfo;
use dao_proposal_sudo::msg::ExecuteMsgFns;
use networks::PION_1;

use crate::arena::Arena;
use crate::tests::helpers::{setup_arena, setup_voting_module, teams_to_members};

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
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&teams),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        LeagueInstantiateExt {
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
            })?,
            label: "League Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["League Rule".to_string()]),
        None,
    )?;

    assert!(res.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|attr| attr.key == "action" && attr.value == "create_competition")));

    // Query the created league
    let league = arena.arena_league_module.competition(Uint128::one())?;
    assert_eq!(league.name, "Test League");

    // Error - attempt to create a league with only one team
    let result = arena.arena_league_module.create_competition(
        "Invalid league",
        Expiration::AtHeight(1000000),
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&[admin.clone()]),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        LeagueInstantiateExt {
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
    );

    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_process_league_matches() -> anyhow::Result<()> {
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

    let teams: Vec<_> = (0..4)
        .map(|i| mock.addr_make_with_balance(format!("team{}", i), coins(10000, DENOM)))
        .collect::<Result<_, _>>()?;

    arena.arena_league_module.set_sender(&admin);

    // Create a league
    let res = arena.arena_league_module.create_competition(
        "A test league".to_string(),
        Expiration::AtHeight(1000000),
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&teams),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        LeagueInstantiateExt {
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
            })?,
            label: "League Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["League Rule".to_string()]),
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

    // Check that the winner has the highest ELO
    let winner_rating = arena
        .arena_core
        .rating(leaderboard[0].member.to_string(), Uint128::one())?
        .unwrap();
    let second_place_rating = arena
        .arena_core
        .rating(leaderboard[1].member.to_string(), Uint128::one())?
        .unwrap();
    let third_place_rating = arena
        .arena_core
        .rating(leaderboard[2].member.to_string(), Uint128::one())?
        .unwrap();
    let fourth_place_rating = arena
        .arena_core
        .rating(leaderboard[3].member.to_string(), Uint128::one())?
        .unwrap();

    assert!(winner_rating.value > second_place_rating.value);
    assert!(second_place_rating.value > third_place_rating.value);
    assert!(third_place_rating.value > fourth_place_rating.value);

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
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&teams),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        LeagueInstantiateExt {
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
            })?,
            label: "League Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["League Rule".to_string()]),
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

    // Get match
    let round_response = arena
        .arena_league_module
        .round(Uint128::one(), Uint64::one())?;
    let match_1 = round_response
        .matches
        .into_iter()
        .find(|x| x.match_number == Uint128::one())
        .expect("Could not get match number 1");

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
        match_1.team_1.to_string(),
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
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&teams),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        LeagueInstantiateExt {
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
            })?,
            label: "Odd League Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Odd League Rule".to_string()]),
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
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&teams),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        LeagueInstantiateExt {
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
            })?,
            label: "Tie League Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Tie League Rule".to_string()]),
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
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&teams),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        LeagueInstantiateExt {
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
            })?,
            label: "Distribution League Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Distribution League Rule".to_string()]),
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
fn test_create_huge_league() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    arena.arena_league_module.set_sender(&admin);

    // Create a league with a ton of members
    // This should create ~500*499/2 (124,750) matches
    let teams: Vec<Addr> = (1..=500)
        .map(|i| mock.addr_make(format!("team{}", i)))
        .collect();
    let result = arena.arena_league_module.create_competition(
        "Huge League",
        Expiration::AtHeight(1000000),
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&teams),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        LeagueInstantiateExt {
            match_win_points: Uint64::new(3),
            match_draw_points: Uint64::new(1),
            match_lose_points: Uint64::zero(),
            distribution: vec![Decimal::percent(100)],
        },
        "Huge League",
        None,
        Some(Uint128::one()),
        None,
        None,
        None,
        None,
    );

    assert!(result.is_ok());

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
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&teams),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        LeagueInstantiateExt {
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
            })?,
            label: "Out of Order League Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Out of Order League Rule".to_string()]),
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
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&teams),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        LeagueInstantiateExt {
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
            })?,
            label: "Multiple Adjustments League Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["Multiple Adjustments League Rule".to_string()]),
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

#[test]
fn test_league_tiebreaking_logic() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let team1 = mock.addr_make_with_balance("team1", coins(10000, DENOM))?;
    let team2 = mock.addr_make_with_balance("team2", coins(10000, DENOM))?;
    let team3 = mock.addr_make_with_balance("team3", coins(10000, DENOM))?;
    let team4 = mock.addr_make_with_balance("team4", coins(10000, DENOM))?;

    arena.arena_league_module.set_sender(&admin);

    // Create a league
    let res = arena.arena_league_module.create_competition(
        "Test League".to_string(),
        Expiration::AtHeight(1000000),
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&[
                        team1.clone(),
                        team2.clone(),
                        team3.clone(),
                        team4.clone(),
                    ]),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        LeagueInstantiateExt {
            match_win_points: Uint64::new(3),
            match_draw_points: Uint64::new(1),
            match_lose_points: Uint64::zero(),
            distribution: vec![
                Decimal::percent(50),
                Decimal::percent(30),
                Decimal::percent(15),
                Decimal::percent(5),
            ],
        },
        "Tiebreaker Test League".to_string(),
        None,
        Some(Uint128::one()),
        Some(EscrowInstantiateInfo {
            code_id: arena.arena_escrow.code_id()?,
            msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                dues: vec![
                    MemberBalanceUnchecked {
                        addr: team1.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    },
                    MemberBalanceUnchecked {
                        addr: team2.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    },
                    MemberBalanceUnchecked {
                        addr: team3.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    },
                    MemberBalanceUnchecked {
                        addr: team4.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    },
                ],
            })?,
            label: "League Escrow".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["League Rule".to_string()]),
        None,
    )?;
    let league_id = Uint128::one();

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
    for team in [&team1, &team2, &team3, &team4] {
        arena.arena_escrow.set_sender(team);
        arena.arena_escrow.receive_native(&coins(1000, DENOM))?;
    }

    // Add stat types
    arena.arena_league_module.update_stat_types(
        league_id,
        vec![
            StatType {
                name: "goal_difference".to_string(),
                value_type: StatValueType::Uint,
                tie_breaker_priority: Some(1),
                is_beneficial: true,
                aggregation_type: None,
            },
            StatType {
                name: "goals_scored".to_string(),
                value_type: StatValueType::Uint,
                tie_breaker_priority: Some(2),
                is_beneficial: true,
                aggregation_type: None,
            },
            StatType {
                name: "fouls".to_string(),
                value_type: StatValueType::Uint,
                tie_breaker_priority: Some(3),
                is_beneficial: false,
                aggregation_type: None,
            },
        ],
        vec![],
    )?;

    // Round 1: Team1 vs Team2 (Draw), Team3 vs Team4 (Draw)
    arena.arena_league_module.process_match(
        league_id,
        vec![
            MatchResultMsg {
                match_number: Uint128::one(),
                match_result: MatchResult::Draw,
            },
            MatchResultMsg {
                match_number: Uint128::new(2),
                match_result: MatchResult::Draw,
            },
        ],
        Uint64::one(),
    )?;
    arena.arena_league_module.input_stats(
        league_id,
        vec![
            MemberStatsMsg {
                addr: team1.to_string(),
                stats: vec![
                    StatMsg::InputStat {
                        name: "goal_difference".to_string(),
                        value: StatValue::Uint(Uint128::new(0)),
                    },
                    StatMsg::InputStat {
                        name: "goals_scored".to_string(),
                        value: StatValue::Uint(Uint128::new(2)),
                    },
                ],
            },
            MemberStatsMsg {
                addr: team2.to_string(),
                stats: vec![
                    StatMsg::InputStat {
                        name: "goal_difference".to_string(),
                        value: StatValue::Uint(Uint128::new(0)),
                    },
                    StatMsg::InputStat {
                        name: "goals_scored".to_string(),
                        value: StatValue::Uint(Uint128::new(2)),
                    },
                ],
            },
            MemberStatsMsg {
                addr: team3.to_string(),
                stats: vec![
                    StatMsg::InputStat {
                        name: "goal_difference".to_string(),
                        value: StatValue::Uint(Uint128::new(0)),
                    },
                    StatMsg::InputStat {
                        name: "goals_scored".to_string(),
                        value: StatValue::Uint(Uint128::new(4)),
                    },
                ],
            },
            MemberStatsMsg {
                addr: team4.to_string(),
                stats: vec![
                    StatMsg::InputStat {
                        name: "goal_difference".to_string(),
                        value: StatValue::Uint(Uint128::new(0)),
                    },
                    StatMsg::InputStat {
                        name: "goals_scored".to_string(),
                        value: StatValue::Uint(Uint128::new(4)),
                    },
                ],
            },
        ],
    )?;

    // Round 2: Team1 vs Team3 (Draw), Team2 vs Team4 (Draw)
    arena.arena_league_module.process_match(
        league_id,
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
    arena.arena_league_module.input_stats(
        league_id,
        vec![
            MemberStatsMsg {
                addr: team1.to_string(),
                stats: vec![
                    StatMsg::InputStat {
                        name: "goal_difference".to_string(),
                        value: StatValue::Uint(Uint128::new(0)),
                    },
                    StatMsg::InputStat {
                        name: "goals_scored".to_string(),
                        value: StatValue::Uint(Uint128::new(3)),
                    },
                ],
            },
            MemberStatsMsg {
                addr: team2.to_string(),
                stats: vec![
                    StatMsg::InputStat {
                        name: "goal_difference".to_string(),
                        value: StatValue::Uint(Uint128::new(0)),
                    },
                    StatMsg::InputStat {
                        name: "goals_scored".to_string(),
                        value: StatValue::Uint(Uint128::new(3)),
                    },
                ],
            },
            MemberStatsMsg {
                addr: team3.to_string(),
                stats: vec![
                    StatMsg::InputStat {
                        name: "goal_difference".to_string(),
                        value: StatValue::Uint(Uint128::new(0)),
                    },
                    StatMsg::InputStat {
                        name: "goals_scored".to_string(),
                        value: StatValue::Uint(Uint128::new(5)),
                    },
                ],
            },
            MemberStatsMsg {
                addr: team4.to_string(),
                stats: vec![
                    StatMsg::InputStat {
                        name: "goal_difference".to_string(),
                        value: StatValue::Uint(Uint128::new(0)),
                    },
                    StatMsg::InputStat {
                        name: "goals_scored".to_string(),
                        value: StatValue::Uint(Uint128::new(5)),
                    },
                ],
            },
        ],
    )?;

    // Round 3: Team1 vs Team4 (Draw), Team3 vs Team2 (Draw)
    arena.arena_league_module.input_stats(
        league_id,
        vec![
            MemberStatsMsg {
                addr: team1.to_string(),
                stats: vec![
                    StatMsg::InputStat {
                        name: "goal_difference".to_string(),
                        value: StatValue::Uint(Uint128::new(0)),
                    },
                    StatMsg::InputStat {
                        name: "goals_scored".to_string(),
                        value: StatValue::Uint(Uint128::new(3)),
                    },
                    StatMsg::InputStat {
                        name: "fouls".to_string(),
                        value: StatValue::Uint(Uint128::new(5)),
                    },
                ],
            },
            MemberStatsMsg {
                addr: team2.to_string(),
                stats: vec![
                    StatMsg::InputStat {
                        name: "goal_difference".to_string(),
                        value: StatValue::Uint(Uint128::new(0)),
                    },
                    StatMsg::InputStat {
                        name: "goals_scored".to_string(),
                        value: StatValue::Uint(Uint128::new(3)),
                    },
                    StatMsg::InputStat {
                        name: "fouls".to_string(),
                        value: StatValue::Uint(Uint128::new(4)),
                    },
                ],
            },
            MemberStatsMsg {
                addr: team3.to_string(),
                stats: vec![
                    StatMsg::InputStat {
                        name: "goal_difference".to_string(),
                        value: StatValue::Uint(Uint128::new(0)),
                    },
                    StatMsg::InputStat {
                        name: "goals_scored".to_string(),
                        value: StatValue::Uint(Uint128::new(5)),
                    },
                    StatMsg::InputStat {
                        name: "fouls".to_string(),
                        value: StatValue::Uint(Uint128::new(0)),
                    },
                ],
            },
            MemberStatsMsg {
                addr: team4.to_string(),
                stats: vec![
                    StatMsg::InputStat {
                        name: "goal_difference".to_string(),
                        value: StatValue::Uint(Uint128::new(0)),
                    },
                    StatMsg::InputStat {
                        name: "goals_scored".to_string(),
                        value: StatValue::Uint(Uint128::new(5)),
                    },
                    StatMsg::InputStat {
                        name: "fouls".to_string(),
                        value: StatValue::Uint(Uint128::new(1)),
                    },
                ],
            },
        ],
    )?;
    arena.arena_league_module.process_match(
        league_id,
        vec![
            MatchResultMsg {
                match_number: Uint128::new(5),
                match_result: MatchResult::Draw,
            },
            MatchResultMsg {
                match_number: Uint128::new(6),
                match_result: MatchResult::Draw,
            },
        ],
        Uint64::new(3),
    )?;

    // Get the final leaderboard
    let leaderboard = arena.arena_league_module.leaderboard(league_id, None)?;

    // Check points
    assert_eq!(leaderboard[0].points, Int128::new(3));
    assert_eq!(leaderboard[1].points, Int128::new(3));
    assert_eq!(leaderboard[2].points, Int128::new(3));
    assert_eq!(leaderboard[3].points, Int128::new(3));

    // Check final distribution
    let team1_balance = arena.arena_escrow.balance(team1.to_string())?;
    let team3_balance = arena.arena_escrow.balance(team3.to_string())?;
    let team2_balance = arena.arena_escrow.balance(team2.to_string())?;
    let team4_balance = arena.arena_escrow.balance(team4.to_string())?;

    assert_eq!(
        team3_balance.unwrap().native.unwrap()[0].amount,
        Uint128::new(1900)
    ); // 50% of 3800 (4000 - 5% tax)
       // Team3: 1st place due to highest goals scored (5) and lowest fouls (0)

    assert_eq!(
        team4_balance.unwrap().native.unwrap()[0].amount,
        Uint128::new(1140)
    ); // 30% of 3800
       // Team4: 2nd place due to highest goals scored (5) and second-lowest fouls (1)

    assert_eq!(
        team2_balance.unwrap().native.unwrap()[0].amount,
        Uint128::new(570)
    ); // 15% of 3800
       // Team2: 3rd place due to lower goals scored (3) than Team3/Team4, but fewer fouls (4) than Team1

    assert_eq!(
        team1_balance.unwrap().native.unwrap()[0].amount,
        Uint128::new(190)
    ); // 5% of 3800
       // Team1: 4th place due to lower goals scored (3) and highest fouls (5)

    // Check DAO balance (5% tax)
    let dao_balance = mock.query_balance(&arena.dao_dao.dao_core.address()?, DENOM)?;
    assert_eq!(dao_balance, Uint128::new(200)); // 5% of 4000

    Ok(())
}

#[test]
fn test_league_tiebreaking_logic_with_aggregates() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let (mut arena, admin) = setup_arena(&mock)?;

    let team1 = mock.addr_make_with_balance("team1", coins(10000, DENOM))?;
    let team2 = mock.addr_make_with_balance("team2", coins(10000, DENOM))?;
    let team3 = mock.addr_make_with_balance("team3", coins(10000, DENOM))?;
    let team4 = mock.addr_make_with_balance("team4", coins(10000, DENOM))?;

    arena.arena_league_module.set_sender(&admin);

    // Create a league
    let res = arena.arena_league_module.create_competition(
        "Test League with Aggregates".to_string(),
        Expiration::AtHeight(1000000),
        GroupContractInfo::New {
            info: ModuleInstantiateInfo {
                code_id: arena.arena_group.code_id()?,
                msg: to_json_binary(&group::InstantiateMsg {
                    members: teams_to_members(&[
                        team1.clone(),
                        team2.clone(),
                        team3.clone(),
                        team4.clone(),
                    ]),
                })?,
                admin: None,
                funds: vec![],
                label: "Arena Group".to_string(),
            },
        },
        LeagueInstantiateExt {
            match_win_points: Uint64::new(3),
            match_draw_points: Uint64::new(1),
            match_lose_points: Uint64::zero(),
            distribution: vec![
                Decimal::percent(50),
                Decimal::percent(30),
                Decimal::percent(15),
                Decimal::percent(5),
            ],
        },
        "Tiebreaker Test League with Aggregates".to_string(),
        None,
        Some(Uint128::one()),
        Some(EscrowInstantiateInfo {
            code_id: arena.arena_escrow.code_id()?,
            msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                dues: vec![
                    MemberBalanceUnchecked {
                        addr: team1.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    },
                    MemberBalanceUnchecked {
                        addr: team2.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    },
                    MemberBalanceUnchecked {
                        addr: team3.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    },
                    MemberBalanceUnchecked {
                        addr: team4.to_string(),
                        balance: BalanceUnchecked {
                            native: Some(vec![Coin::new(1000, DENOM)]),
                            cw20: None,
                            cw721: None,
                        },
                    },
                ],
            })?,
            label: "League Escrow with Aggregates".to_string(),
            additional_layered_fees: None,
        }),
        None,
        Some(vec!["League Rule".to_string()]),
        None,
    )?;
    let league_id = Uint128::one();

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
    for team in [&team1, &team2, &team3, &team4] {
        arena.arena_escrow.set_sender(team);
        arena.arena_escrow.receive_native(&coins(1000, DENOM))?;
    }

    // Add stat types with aggregation
    arena.arena_league_module.update_stat_types(
        league_id,
        vec![
            StatType {
                name: "total_goals".to_string(),
                value_type: StatValueType::Uint,
                tie_breaker_priority: Some(1),
                is_beneficial: true,
                aggregation_type: Some(StatAggregationType::Cumulative),
            },
            StatType {
                name: "average_possession".to_string(),
                value_type: StatValueType::Decimal,
                tie_breaker_priority: Some(2),
                is_beneficial: true,
                aggregation_type: Some(StatAggregationType::Average),
            },
            StatType {
                name: "total_fouls".to_string(),
                value_type: StatValueType::Uint,
                tie_breaker_priority: Some(3),
                is_beneficial: false,
                aggregation_type: Some(StatAggregationType::Cumulative),
            },
        ],
        vec![],
    )?;

    // Simulate 3 rounds of matches
    for round in 1..=3u64 {
        arena.arena_league_module.input_stats(
            league_id,
            vec![
                MemberStatsMsg {
                    addr: team1.to_string(),
                    stats: vec![
                        StatMsg::InputStat {
                            name: "total_goals".to_string(),
                            value: StatValue::Uint(Uint128::new(2)),
                        },
                        StatMsg::InputStat {
                            name: "average_possession".to_string(),
                            value: StatValue::Decimal(Decimal::percent(55)),
                        },
                        StatMsg::InputStat {
                            name: "total_fouls".to_string(),
                            value: StatValue::Uint(Uint128::new(3)),
                        },
                    ],
                },
                MemberStatsMsg {
                    addr: team2.to_string(),
                    stats: vec![
                        StatMsg::InputStat {
                            name: "total_goals".to_string(),
                            value: StatValue::Uint(Uint128::new(2)),
                        },
                        StatMsg::InputStat {
                            name: "average_possession".to_string(),
                            value: StatValue::Decimal(Decimal::percent(50)),
                        },
                        StatMsg::InputStat {
                            name: "total_fouls".to_string(),
                            value: StatValue::Uint(Uint128::new(2)),
                        },
                    ],
                },
                MemberStatsMsg {
                    addr: team3.to_string(),
                    stats: vec![
                        StatMsg::InputStat {
                            name: "total_goals".to_string(),
                            value: StatValue::Uint(Uint128::new(1)),
                        },
                        StatMsg::InputStat {
                            name: "average_possession".to_string(),
                            value: StatValue::Decimal(Decimal::percent(60)),
                        },
                        StatMsg::InputStat {
                            name: "total_fouls".to_string(),
                            value: StatValue::Uint(Uint128::new(1)),
                        },
                    ],
                },
                MemberStatsMsg {
                    addr: team4.to_string(),
                    stats: vec![
                        StatMsg::InputStat {
                            name: "total_goals".to_string(),
                            value: StatValue::Uint(Uint128::new(1)),
                        },
                        StatMsg::InputStat {
                            name: "average_possession".to_string(),
                            value: StatValue::Decimal(Decimal::percent(45)),
                        },
                        StatMsg::InputStat {
                            name: "total_fouls".to_string(),
                            value: StatValue::Uint(Uint128::new(4)),
                        },
                    ],
                },
            ],
        )?;
        arena.arena_league_module.process_match(
            league_id,
            vec![
                MatchResultMsg {
                    match_number: Uint128::new(2 * round as u128 - 1),
                    match_result: MatchResult::Draw,
                },
                MatchResultMsg {
                    match_number: Uint128::new(2 * round as u128),
                    match_result: MatchResult::Draw,
                },
            ],
            Uint64::new(round),
        )?;
        mock.next_block()?;
    }

    // Get the final leaderboard
    let leaderboard = arena.arena_league_module.leaderboard(league_id, None)?;

    // Check points (all teams should have 3 points from 3 draws)
    for team in &leaderboard {
        assert_eq!(team.points, Int128::new(3));
    }

    // Check final distribution
    let team1_balance = arena.arena_escrow.balance(team1.to_string())?;
    let team2_balance = arena.arena_escrow.balance(team2.to_string())?;
    let team3_balance = arena.arena_escrow.balance(team3.to_string())?;
    let team4_balance = arena.arena_escrow.balance(team4.to_string())?;

    // Expected order: team1, team2, team3, team4
    // team1: Highest total goals (6), highest average possession (55%)
    // team2: Tied total goals with team1 (6), lower average possession (50%), but fewer total fouls (6) than team1 (9)
    // team3: Lower total goals (3), but highest average possession (60%) and lowest total fouls (3)
    // team4: Lowest in all categories

    assert_eq!(
        team1_balance.unwrap().native.unwrap()[0].amount,
        Uint128::new(1900)
    ); // 50% of 3800 (4000 - 5% tax)
    assert_eq!(
        team2_balance.unwrap().native.unwrap()[0].amount,
        Uint128::new(1140)
    ); // 30% of 3800
    assert_eq!(
        team3_balance.unwrap().native.unwrap()[0].amount,
        Uint128::new(570)
    ); // 15% of 3800
    assert_eq!(
        team4_balance.unwrap().native.unwrap()[0].amount,
        Uint128::new(190)
    ); // 5% of 3800

    // Check DAO balance (5% tax)
    let dao_balance = mock.query_balance(&arena.dao_dao.dao_core.address()?, DENOM)?;
    assert_eq!(dao_balance, Uint128::new(200)); // 5% of 4000

    // Verify aggregated stats
    let stats_table = arena
        .arena_league_module
        .stats_table(Uint128::one(), None, None)?;
    for team in [&team1, &team2, &team3, &team4] {
        let stats = stats_table
            .iter()
            .find(|x| x.addr == team)
            .unwrap()
            .stats
            .clone();
        let total_goals = stats.iter().find(|s| s.name() == "total_goals").unwrap();
        let average_possession = stats
            .iter()
            .find(|s| s.name() == "average_possession")
            .unwrap();
        let total_fouls = stats.iter().find(|s| s.name() == "total_fouls").unwrap();

        if team == team1 {
            assert_eq!(*total_goals.value(), StatValue::Uint(Uint128::new(6)));
            assert_eq!(
                *average_possession.value(),
                StatValue::Decimal(Decimal::percent(55))
            );
            assert_eq!(*total_fouls.value(), StatValue::Uint(Uint128::new(9)));
        } else if team == team2 {
            assert_eq!(*total_goals.value(), StatValue::Uint(Uint128::new(6)));
            assert_eq!(
                *average_possession.value(),
                StatValue::Decimal(Decimal::percent(50))
            );
            assert_eq!(*total_fouls.value(), StatValue::Uint(Uint128::new(6)));
        } else if team == team3 {
            assert_eq!(*total_goals.value(), StatValue::Uint(Uint128::new(3)));
            assert_eq!(
                *average_possession.value(),
                StatValue::Decimal(Decimal::percent(60))
            );
            assert_eq!(*total_fouls.value(), StatValue::Uint(Uint128::new(3)));
        } else if team == team4 {
            assert_eq!(*total_goals.value(), StatValue::Uint(Uint128::new(3)));
            assert_eq!(
                *average_possession.value(),
                StatValue::Decimal(Decimal::percent(45))
            );
            assert_eq!(*total_fouls.value(), StatValue::Uint(Uint128::new(12)));
        }
    }

    Ok(())
}

#[test]
fn test_migration_v2_v2_1() -> anyhow::Result<()> {
    let app = CloneTesting::new(PION_1)?;
    let mut arena = Arena::new(app.clone());
    const ARENA_DAO: &str = "neutron1ehkcl0n6s2jtdw75xsvfxm304mz4hs5z7jt6wn5mk0celpj0epqql4ulxk";
    let arena_dao_addr = Addr::unchecked(ARENA_DAO);

    arena.arena_group.upload()?;
    arena.arena_league_module.upload()?;

    arena.arena_group.instantiate(
        &group::InstantiateMsg { members: None },
        Some(&arena_dao_addr),
        None,
    )?;

    arena.arena_league_module.set_address(&Addr::unchecked(
        "neutron1pzh32kr9r4gl0fcgrp69f4us2he5zysfzta096lg932fu6qr4s6srk8atv",
    ));
    arena.arena_league_module.set_sender(&arena_dao_addr);

    arena.arena_league_module.migrate(
        &MigrateMsg::WithGroupAddress {
            group_contract: arena.arena_group.addr_str()?,
        },
        arena.arena_league_module.code_id()?,
    )?;

    Ok(())
}
