use std::str::FromStr;

use arena_interface::competition::msg::{EscrowInstantiateInfo, ModuleInfo};
use arena_interface::core::{CompetitionModuleQuery, CompetitionModuleResponse, QueryExt};
use arena_interface::fees::FeeInformation;
use arena_league_module::{
    msg::{
        ExecuteExt, ExecuteMsg, InstantiateMsg, LeagueInstantiateExt, LeagueQueryExt,
        LeagueResponse, MatchResultMsg, MemberPoints, QueryMsg, RoundResponse,
    },
    state::{Match, MatchResult, PointAdjustment},
};
use cosmwasm_std::{
    coins, to_json_binary, Addr, Coin, Coins, Decimal, Empty, Int128, Uint128, Uint64, WasmMsg,
};
use cw4::Member;
use cw_balance::{BalanceUnchecked, MemberBalanceUnchecked};
use cw_multi_test::{next_block, App, AppResponse, BankKeeper, Executor, MockApiBech32};
use cw_utils::Expiration;
use dao_interface::state::ModuleInstantiateInfo;

use crate::tests::{
    app::{get_app, set_balances},
    core::{get_attr_value, setup_core_context, ADMIN},
};

use super::core::CoreContext;

struct Context {
    app: App<BankKeeper, MockApiBech32>,
    core: CoreContext,
    league: LeagueContext,
}

pub struct LeagueContext {
    pub league_module_addr: Addr,
    pub escrow_id: u64,
}

fn setup_league_context(
    app: &mut App<BankKeeper, MockApiBech32>,
    core_context: &CoreContext,
) -> LeagueContext {
    let league_module_id = app.store_code(arena_testing::contracts::arena_league_module_contract());
    let escrow_id = app.store_code(arena_testing::contracts::arena_dao_escrow_contract());

    // Attach the arena-league-module to the arena-core
    let result = app.execute_contract(
        app.api().addr_make(ADMIN),
        core_context.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_context.arena_core_addr.to_string(),
                funds: vec![],
                msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
                    msg: arena_interface::core::ExecuteExt::UpdateCompetitionModules {
                        to_add: vec![ModuleInstantiateInfo {
                            code_id: league_module_id,
                            msg: to_json_binary(&InstantiateMsg {
                                key: "Leagues".to_string(),
                                description: "This is a description".to_string(),
                                extension: Empty {},
                            })
                            .unwrap(),
                            admin: None,
                            label: "arena-league-module".to_string(),
                        }],
                        to_disable: vec![],
                    },
                })
                .unwrap(),
            }
            .into()],
        },
        &[],
    );
    assert!(result.is_ok());
    app.update_block(next_block);

    // Get the league module
    let league_module = app
        .wrap()
        .query_wasm_smart::<CompetitionModuleResponse<Addr>>(
            core_context.arena_core_addr.clone(),
            &arena_interface::core::QueryMsg::QueryExtension {
                msg: QueryExt::CompetitionModule {
                    query: CompetitionModuleQuery::Key("Leagues".to_string(), None),
                },
            },
        )
        .unwrap();

    LeagueContext {
        league_module_addr: league_module.addr,
        escrow_id,
    }
}

fn create_competition(
    context: &mut Context,
    expiration: Expiration,
    members: Vec<cw4::Member>,
    dues: Option<Vec<MemberBalanceUnchecked>>,
    distribution: Vec<Decimal>,
    additional_layered_fees: Option<Vec<FeeInformation<String>>>,
) -> cw_multi_test::error::AnyResult<AppResponse> {
    let teams: Vec<String> = members.iter().map(|x| x.addr.to_string()).collect();

    context.app.execute_contract(
        context.app.api().addr_make(ADMIN),
        context.league.league_module_addr.clone(),
        &ExecuteMsg::CreateCompetition {
            category_id: Some(Uint128::one()),
            host: ModuleInfo::New {
                info: ModuleInstantiateInfo {
                    code_id: context.core.dao_core_id,
                    msg: to_json_binary(&super::helpers::get_competition_dao_instantiate_msg(
                        context.core.cw4_id,
                        context.core.cw4_voting_module_id,
                        context.core.dao_proposal_single_id,
                        dao_proposal_single::msg::InstantiateMsg {
                            threshold: dao_voting::threshold::Threshold::AbsolutePercentage {
                                percentage: dao_voting::threshold::PercentageThreshold::Majority {},
                            },
                            min_voting_period: None,
                            max_voting_period: cw_utils_v16::Duration::Height(10u64),
                            only_members_execute: false,
                            allow_revoting: false,
                            pre_propose_info:
                                dao_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {},
                            close_proposal_on_execution_failure: true,
                        },
                        members,
                    ))
                    .unwrap(),
                    admin: None,
                    label: "DAO".to_owned(),
                },
            },
            escrow: dues.map(|x| EscrowInstantiateInfo {
                code_id: context.league.escrow_id,
                msg: to_json_binary(&arena_interface::escrow::InstantiateMsg {
                    dues: x,
                    should_activate_on_funded: None,
                })
                .unwrap(),
                label: "Escrow".to_owned(),
                additional_layered_fees,
            }),
            name: "This is a competition name".to_string(),
            description: "This is a description".to_string(),
            expiration,
            rules: vec![
                "Rule 1".to_string(),
                "Rule 2".to_string(),
                "Rule 3".to_string(),
            ],
            rulesets: vec![],
            banner: None,
            should_activate_on_funded: None,
            instantiate_extension: LeagueInstantiateExt {
                teams,
                match_win_points: Uint64::from(3u64),
                match_draw_points: Uint64::one(),
                match_lose_points: Uint64::zero(),
                distribution,
            },
        },
        &[],
    )
}

fn get_id_from_competition_creation_result(
    result: cw_multi_test::error::AnyResult<AppResponse>,
) -> Uint128 {
    let id = get_attr_value(&result.unwrap(), "competition_id");
    assert!(id.is_some());

    let result = Uint128::from_str(&id.unwrap());
    assert!(result.is_ok());

    result.unwrap()
}

#[test]
fn test_league_validation() {
    let mut app = get_app();
    let users = [
        app.api().addr_make("user1"),
        app.api().addr_make("user2"),
        app.api().addr_make("user3"),
        app.api().addr_make("user4"),
        app.api().addr_make("user5"),
    ];
    let admin = app.api().addr_make(ADMIN);

    let core_context = setup_core_context(
        &mut app,
        vec![Member {
            addr: admin.to_string(),
            weight: 1u64,
        }],
    );
    let league_context = setup_league_context(&mut app, &core_context);
    let mut context = Context {
        app,
        core: core_context,
        league: league_context,
    };

    let members: Vec<_> = users
        .iter()
        .map(|x| Member {
            addr: x.to_string(),
            weight: 1u64,
        })
        .collect();

    // First show a successful distribution
    let result = create_competition(
        &mut context,
        Expiration::Never {},
        members.clone(),
        None,
        vec![
            Decimal::from_ratio(70u128, 100u128),
            Decimal::from_ratio(20u128, 100u128),
            Decimal::from_ratio(10u128, 100u128),
        ],
        None,
    );
    assert!(result.is_ok());

    // Error on distributions not summing to 1
    let result = create_competition(
        &mut context,
        Expiration::Never {},
        members.clone(),
        None,
        vec![
            Decimal::from_ratio(70u128, 100u128),
            Decimal::from_ratio(20u128, 100u128),
            Decimal::from_ratio(5u128, 100u128),
        ],
        None,
    );
    assert!(result.is_err());

    // Error on distributions greater than teams size
    let result = create_competition(
        &mut context,
        Expiration::Never {},
        members.clone(),
        None,
        vec![
            Decimal::from_ratio(15u128, 100u128),
            Decimal::from_ratio(15u128, 100u128),
            Decimal::from_ratio(15u128, 100u128),
            Decimal::from_ratio(15u128, 100u128),
            Decimal::from_ratio(15u128, 100u128),
            Decimal::from_ratio(15u128, 100u128),
            Decimal::from_ratio(10u128, 100u128),
        ],
        None,
    );
    assert!(result.is_err());

    // Error on teams not greater than 1
    let result = create_competition(
        &mut context,
        Expiration::Never {},
        vec![members[0].clone()],
        None,
        vec![Decimal::one()],
        None,
    );
    assert!(result.is_err());

    // Error on teams not unique
    let result = create_competition(
        &mut context,
        Expiration::Never {},
        vec![members[0].clone(), members[0].clone()],
        None,
        vec![Decimal::one()],
        None,
    );
    assert!(result.is_err());

    // Test 4 members
    context.app.update_block(|x| x.height += 10); // Need to update block for instantiate2 to work
    let result = create_competition(
        &mut context,
        Expiration::Never {},
        members[0..members.len() - 1].to_vec(),
        None,
        vec![Decimal::one()],
        None,
    );
    assert!(result.is_ok());
    let response = result.unwrap();

    assert_eq!(get_attr_value(&response, "rounds"), Some("3".to_string()));
    assert_eq!(get_attr_value(&response, "matches"), Some("6".to_string()));
    assert_eq!(get_attr_value(&response, "teams"), Some("4".to_string()));
}

#[test]
fn test_leagues() {
    let mut app = get_app();
    let users = [
        app.api().addr_make("user1"),
        app.api().addr_make("user2"),
        app.api().addr_make("user3"),
        app.api().addr_make("user4"),
        app.api().addr_make("user5"),
    ];
    let admin = app.api().addr_make(ADMIN);
    let wager_amount_uint128 = Uint128::from(10_000u128);
    let wager_amount = format!("{}{}", wager_amount_uint128, "juno");

    set_balances(
        &mut app,
        vec![
            (users[0].clone(), Coins::from_str(&wager_amount).unwrap()),
            (users[1].clone(), Coins::from_str(&wager_amount).unwrap()),
        ],
    );

    let core_context = setup_core_context(
        &mut app,
        vec![Member {
            addr: admin.to_string(),
            weight: 1u64,
        }],
    );
    let league_context = setup_league_context(&mut app, &core_context);
    let mut context = Context {
        app,
        core: core_context,
        league: league_context,
    };

    // Create competition
    let starting_height = context.app.block_info().height;
    let result = create_competition(
        &mut context,
        Expiration::AtHeight(starting_height + 100),
        users
            .iter()
            .map(|x| Member {
                addr: x.to_string(),
                weight: 1u64,
            })
            .collect(),
        Some(vec![
            MemberBalanceUnchecked {
                addr: users[0].to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
            MemberBalanceUnchecked {
                addr: users[1].to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
        ]),
        vec![
            Decimal::from_ratio(70u128, 100u128),
            Decimal::from_ratio(20u128, 100u128),
            Decimal::from_ratio(10u128, 100u128),
        ],
        None,
    );
    assert!(result.is_ok());
    let competition_id = get_id_from_competition_creation_result(result);

    // Get competition
    let competition: LeagueResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::Competition { competition_id },
        )
        .unwrap();
    assert_eq!(competition.extension.rounds, Uint64::from(5u64));
    assert_eq!(competition.extension.matches, Uint128::from(10u64));

    // Get round 1
    let round1: RoundResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::QueryExtension {
                msg: LeagueQueryExt::Round {
                    league_id: competition_id,
                    round_number: Uint64::one(),
                },
            },
        )
        .unwrap();
    assert_eq!(
        round1,
        RoundResponse {
            round_number: Uint64::one(),
            matches: vec![
                Match {
                    match_number: Uint128::from(2u128),
                    team_1: users[1].clone(),
                    team_2: users[4].clone(),
                    result: None
                },
                Match {
                    match_number: Uint128::one(),
                    team_1: users[0].clone(),
                    team_2: users[3].clone(),
                    result: None
                }
            ],
        },
    );

    // Get round 2
    let round1: RoundResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::QueryExtension {
                msg: LeagueQueryExt::Round {
                    league_id: competition_id,
                    round_number: Uint64::from(2u64),
                },
            },
        )
        .unwrap();
    assert_eq!(
        round1,
        RoundResponse {
            round_number: Uint64::from(2u64),
            matches: vec![
                Match {
                    match_number: Uint128::from(4u128),
                    team_1: users[1].clone(),
                    team_2: users[2].clone(),
                    result: None
                },
                Match {
                    match_number: Uint128::from(3u128),
                    team_1: users[0].clone(),
                    team_2: users[4].clone(),
                    result: None
                }
            ],
        },
    );

    context.app.update_block(|x| x.height += 10);

    // Check that matches can't be processed until the league is active
    let msg = dao_proposal_sudo::msg::ExecuteMsg::Execute {
        msgs: vec![WasmMsg::Execute {
            contract_addr: context.league.league_module_addr.to_string(),
            msg: to_json_binary(&ExecuteMsg::Extension {
                msg: ExecuteExt::ProcessMatch {
                    league_id: competition_id,
                    round_number: Uint64::one(),
                    match_results: vec![
                        MatchResultMsg {
                            match_number: Uint128::one(),
                            match_result: MatchResult::Team1,
                        },
                        MatchResultMsg {
                            match_number: Uint128::from(2u128),
                            match_result: MatchResult::Draw,
                        },
                    ],
                },
            })
            .unwrap(),
            funds: vec![],
        }
        .into()],
    };
    let result = context.app.execute_contract(
        admin.clone(),
        context.core.sudo_proposal_addr.clone(),
        &msg,
        &[],
    );
    assert!(result.is_err());

    // Fund escrow
    context
        .app
        .execute_contract(
            users[0].clone(),
            competition.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();
    context
        .app
        .execute_contract(
            users[1].clone(),
            competition.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();

    // Check that we can now process
    let result = context.app.execute_contract(
        admin.clone(),
        context.core.sudo_proposal_addr.clone(),
        &msg,
        &[],
    );
    assert!(result.is_ok());

    // Check that processed matches was updated
    let competition: LeagueResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::Competition { competition_id },
        )
        .unwrap();
    assert_eq!(competition.extension.processed_matches, Uint128::from(2u64));

    let leaderboard: Vec<MemberPoints> = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::QueryExtension {
                msg: LeagueQueryExt::Leaderboard {
                    league_id: competition_id,
                    round: None,
                },
            },
        )
        .unwrap();

    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[1]).unwrap(),
        MemberPoints {
            member: users[1].clone(),
            points: Int128::one(),
            matches_played: Uint64::one()
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[0]).unwrap(),
        MemberPoints {
            member: users[0].clone(),
            points: Int128::from(3),
            matches_played: Uint64::one()
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[3]).unwrap(),
        MemberPoints {
            member: users[3].clone(),
            points: Int128::zero(),
            matches_played: Uint64::one()
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[4]).unwrap(),
        MemberPoints {
            member: users[4].clone(),
            points: Int128::one(),
            matches_played: Uint64::one()
        }
    );

    context.app.update_block(|x| x.height += 10);

    // Owners realized that users[1] was cheating and have deducted 5 points
    let result = context.app.execute_contract(
        admin.clone(),
        context.core.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: context.league.league_module_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::Extension {
                    msg: ExecuteExt::AddPointAdjustments {
                        league_id: competition_id,
                        addr: users[1].to_string(),
                        point_adjustments: vec![PointAdjustment {
                            description: "Team was caught cheating".to_string(),
                            amount: Int128::new(-3),
                        }],
                    },
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    );
    assert!(result.is_ok());

    // Check users[1] new points
    let leaderboard: Vec<MemberPoints> = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::QueryExtension {
                msg: LeagueQueryExt::Leaderboard {
                    league_id: competition_id,
                    round: None,
                },
            },
        )
        .unwrap();
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[1]).unwrap(),
        MemberPoints {
            member: users[1].clone(),
            points: Int128::from(-2),
            matches_played: Uint64::one()
        }
    );

    // Process 2nd round of matches
    let result = context.app.execute_contract(
        admin.clone(),
        context.core.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: context.league.league_module_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::Extension {
                    msg: ExecuteExt::ProcessMatch {
                        league_id: competition_id,
                        round_number: Uint64::from(2u64),
                        match_results: vec![
                            MatchResultMsg {
                                match_number: Uint128::from(3u128),
                                match_result: MatchResult::Team1,
                            },
                            MatchResultMsg {
                                match_number: Uint128::from(4u128),
                                match_result: MatchResult::Team1,
                            },
                        ],
                    },
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    );
    assert!(result.is_ok());

    let leaderboard: Vec<MemberPoints> = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::QueryExtension {
                msg: LeagueQueryExt::Leaderboard {
                    league_id: competition_id,
                    round: None,
                },
            },
        )
        .unwrap();

    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[0]).unwrap(),
        MemberPoints {
            member: users[0].clone(),
            points: Int128::from(6),
            matches_played: Uint64::from(2u64)
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[1]).unwrap(),
        MemberPoints {
            member: users[1].clone(),
            points: Int128::from(1),
            matches_played: Uint64::from(2u64)
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[2]).unwrap(),
        MemberPoints {
            member: users[2].clone(),
            points: Int128::zero(),
            matches_played: Uint64::one()
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[3]).unwrap(),
        MemberPoints {
            member: users[3].clone(),
            points: Int128::zero(),
            matches_played: Uint64::one()
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[4]).unwrap(),
        MemberPoints {
            member: users[4].clone(),
            points: Int128::one(),
            matches_played: Uint64::from(2u64)
        }
    );

    context.app.update_block(|x| x.height += 10);
    // Process 3rd round of matches
    let result = context.app.execute_contract(
        admin.clone(),
        context.core.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: context.league.league_module_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::Extension {
                    msg: ExecuteExt::ProcessMatch {
                        league_id: competition_id,
                        round_number: Uint64::from(3u64),
                        match_results: vec![
                            MatchResultMsg {
                                match_number: Uint128::from(5u128),
                                match_result: MatchResult::Team1,
                            },
                            MatchResultMsg {
                                match_number: Uint128::from(6u128),
                                match_result: MatchResult::Team1,
                            },
                        ],
                    },
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    );
    assert!(result.is_ok());

    let leaderboard: Vec<MemberPoints> = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::QueryExtension {
                msg: LeagueQueryExt::Leaderboard {
                    league_id: competition_id,
                    round: None,
                },
            },
        )
        .unwrap();

    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[0]).unwrap(),
        MemberPoints {
            member: users[0].clone(),
            points: Int128::from(6),
            matches_played: Uint64::from(2u64)
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[1]).unwrap(),
        MemberPoints {
            member: users[1].clone(),
            points: Int128::from(1),
            matches_played: Uint64::from(3u64)
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[2]).unwrap(),
        MemberPoints {
            member: users[2].clone(),
            points: Int128::zero(),
            matches_played: Uint64::from(2u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[3]).unwrap(),
        MemberPoints {
            member: users[3].clone(),
            points: Int128::from(3),
            matches_played: Uint64::from(2u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[4]).unwrap(),
        MemberPoints {
            member: users[4].clone(),
            points: Int128::from(4),
            matches_played: Uint64::from(3u64)
        }
    );

    context.app.update_block(|x| x.height += 10);
    // Process 4th round of matches
    let result = context.app.execute_contract(
        admin.clone(),
        context.core.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: context.league.league_module_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::Extension {
                    msg: ExecuteExt::ProcessMatch {
                        league_id: competition_id,
                        round_number: Uint64::from(4u64),
                        match_results: vec![
                            MatchResultMsg {
                                match_number: Uint128::from(7u128),
                                match_result: MatchResult::Team1,
                            },
                            MatchResultMsg {
                                match_number: Uint128::from(8u128),
                                match_result: MatchResult::Team1,
                            },
                        ],
                    },
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    );
    assert!(result.is_ok());

    let leaderboard: Vec<MemberPoints> = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::QueryExtension {
                msg: LeagueQueryExt::Leaderboard {
                    league_id: competition_id,
                    round: None,
                },
            },
        )
        .unwrap();

    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[0]).unwrap(),
        MemberPoints {
            member: users[0].clone(),
            points: Int128::from(9),
            matches_played: Uint64::from(3u64)
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[1]).unwrap(),
        MemberPoints {
            member: users[1].clone(),
            points: Int128::from(1),
            matches_played: Uint64::from(3u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[2]).unwrap(),
        MemberPoints {
            member: users[2].clone(),
            points: Int128::zero(),
            matches_played: Uint64::from(3u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[3]).unwrap(),
        MemberPoints {
            member: users[3].clone(),
            points: Int128::from(3),
            matches_played: Uint64::from(3u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[4]).unwrap(),
        MemberPoints {
            member: users[4].clone(),
            points: Int128::from(7),
            matches_played: Uint64::from(4u64)
        }
    );

    context.app.update_block(|x| x.height += 10);
    // Process 5th round of matches
    let result = context.app.execute_contract(
        admin.clone(),
        context.core.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: context.league.league_module_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::Extension {
                    msg: ExecuteExt::ProcessMatch {
                        league_id: competition_id,
                        round_number: Uint64::from(5u64),
                        match_results: vec![
                            MatchResultMsg {
                                match_number: Uint128::from(9u128),
                                match_result: MatchResult::Team1,
                            },
                            MatchResultMsg {
                                match_number: Uint128::from(10u128),
                                match_result: MatchResult::Team2,
                            },
                        ],
                    },
                })
                .unwrap(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    );
    assert!(result.is_ok());

    let leaderboard: Vec<MemberPoints> = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::QueryExtension {
                msg: LeagueQueryExt::Leaderboard {
                    league_id: competition_id,
                    round: None,
                },
            },
        )
        .unwrap();

    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[0]).unwrap(),
        MemberPoints {
            member: users[0].clone(),
            points: Int128::from(12),
            matches_played: Uint64::from(4u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[1]).unwrap(),
        MemberPoints {
            member: users[1].clone(),
            points: Int128::from(1),
            matches_played: Uint64::from(4u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[2]).unwrap(),
        MemberPoints {
            member: users[2].clone(),
            points: Int128::from(0),
            matches_played: Uint64::from(4u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[3]).unwrap(),
        MemberPoints {
            member: users[3].clone(),
            points: Int128::from(6),
            matches_played: Uint64::from(4u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[4]).unwrap(),
        MemberPoints {
            member: users[4].clone(),
            points: Int128::from(7),
            matches_played: Uint64::from(4u64)
        }
    );

    // Check escrow balance was distributed after all matches were processed
    // Distribution was 1st - 70%, 2nd - 20%, 3rd - 10%
    let balances: Vec<MemberBalanceUnchecked> = context
        .app
        .wrap()
        .query_wasm_smart(
            competition.escrow.as_ref().unwrap(),
            &arena_interface::escrow::QueryMsg::Balances {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(
        balances,
        vec![
            MemberBalanceUnchecked {
                addr: users[3].to_string(), // 3rd
                balance: BalanceUnchecked {
                    native: coins(1700, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            },
            MemberBalanceUnchecked {
                addr: users[0].to_string(), // 2nd
                balance: BalanceUnchecked {
                    native: coins(11900, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            },
            MemberBalanceUnchecked {
                addr: users[4].to_string(), // 1st 20000 (prize pool) * .85 (tax) * .7 (1st place earnings) = 11900
                balance: BalanceUnchecked {
                    native: coins(3400, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            }
        ]
    )
}

/// What if all matches are tied or many are tied for a position? Test the optimization for this.
/// Extra placement percentages should be evenly distributed into the percentages
#[test]
fn test_distributions() {
    let mut app = get_app();
    let users = [
        app.api().addr_make("user1"),
        app.api().addr_make("user2"),
        app.api().addr_make("user3"),
        app.api().addr_make("user4"),
        app.api().addr_make("user5"),
    ];
    let admin = app.api().addr_make(ADMIN);
    let wager_amount_uint128 = Uint128::from(10_000u128);
    let wager_amount = format!("{}{}", wager_amount_uint128, "juno");

    set_balances(
        &mut app,
        vec![
            (users[0].clone(), Coins::from_str("100000juno").unwrap()),
            (users[1].clone(), Coins::from_str("100000juno").unwrap()),
        ],
    );

    let core_context = setup_core_context(
        &mut app,
        vec![Member {
            addr: admin.to_string(),
            weight: 1u64,
        }],
    );
    let league_context = setup_league_context(&mut app, &core_context);
    let mut context = Context {
        app,
        core: core_context,
        league: league_context,
    };

    // Create competition
    let starting_height = context.app.block_info().height;
    let result = create_competition(
        &mut context,
        Expiration::AtHeight(starting_height + 100),
        users
            .iter()
            .map(|x| Member {
                addr: x.to_string(),
                weight: 1u64,
            })
            .collect(),
        Some(vec![
            MemberBalanceUnchecked {
                addr: users[0].to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
            MemberBalanceUnchecked {
                addr: users[1].to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
        ]),
        vec![
            Decimal::from_ratio(70u128, 100u128),
            Decimal::from_ratio(20u128, 100u128),
            Decimal::from_ratio(10u128, 100u128),
        ],
        None,
    );
    assert!(result.is_ok());
    let competition_id = get_id_from_competition_creation_result(result);

    // Get the competition
    let competition: LeagueResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::Competition { competition_id },
        )
        .unwrap();

    // Fund escrow
    context
        .app
        .execute_contract(
            users[0].clone(),
            competition.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();
    context
        .app
        .execute_contract(
            users[1].clone(),
            competition.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();

    context.app.update_block(|x| x.height += 10);
    // Process all matches
    let result = context.app.execute_contract(
        admin.clone(),
        context.core.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(1u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(1u128),
                                    match_result: MatchResult::Draw,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(2u128),
                                    match_result: MatchResult::Draw,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(2u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(3u128),
                                    match_result: MatchResult::Draw,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(4u128),
                                    match_result: MatchResult::Draw,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(3u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(5u128),
                                    match_result: MatchResult::Draw,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(6u128),
                                    match_result: MatchResult::Draw,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(4u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(7u128),
                                    match_result: MatchResult::Draw,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(8u128),
                                    match_result: MatchResult::Draw,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(5u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(9u128),
                                    match_result: MatchResult::Draw,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(10u128),
                                    match_result: MatchResult::Draw,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
            ],
        },
        &[],
    );
    assert!(result.is_ok());

    // Check escrow balance was distributed after all matches were processed
    // Distribution was 1st - 70%, 2nd - 20%, 3rd - 10%, but all members tied so it's 20% for everyone
    let balances: Vec<MemberBalanceUnchecked> = context
        .app
        .wrap()
        .query_wasm_smart(
            competition.escrow.as_ref().unwrap(),
            &arena_interface::escrow::QueryMsg::Balances {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(
        balances,
        vec![
            MemberBalanceUnchecked {
                addr: users[3].to_string(),
                balance: BalanceUnchecked {
                    native: coins(3400, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            },
            MemberBalanceUnchecked {
                addr: users[0].to_string(),
                balance: BalanceUnchecked {
                    native: coins(3400, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            },
            MemberBalanceUnchecked {
                addr: users[4].to_string(),
                balance: BalanceUnchecked {
                    native: coins(3400, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            },
            MemberBalanceUnchecked {
                addr: users[2].to_string(),
                balance: BalanceUnchecked {
                    native: coins(3400, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            },
            MemberBalanceUnchecked {
                addr: users[1].to_string(),
                balance: BalanceUnchecked {
                    native: coins(3400, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            }
        ]
    );

    // Now setup the next league for tied winners
    let result = create_competition(
        &mut context,
        Expiration::AtHeight(starting_height + 100),
        users
            .iter()
            .map(|x| Member {
                addr: x.to_string(),
                weight: 1u64,
            })
            .collect(),
        Some(vec![
            MemberBalanceUnchecked {
                addr: users[0].to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
            MemberBalanceUnchecked {
                addr: users[1].to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
        ]),
        vec![
            Decimal::from_ratio(70u128, 100u128),
            Decimal::from_ratio(20u128, 100u128),
            Decimal::from_ratio(10u128, 100u128),
        ],
        None,
    );
    assert!(result.is_ok());
    let competition_id = get_id_from_competition_creation_result(result);

    // Get the competition
    let competition: LeagueResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::Competition { competition_id },
        )
        .unwrap();

    // Fund escrow
    context
        .app
        .execute_contract(
            users[0].clone(),
            competition.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();
    context
        .app
        .execute_contract(
            users[1].clone(),
            competition.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();

    context.app.update_block(|x| x.height += 10);
    // Process all matches - 2 wins then all draws
    let result = context.app.execute_contract(
        admin.clone(),
        context.core.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(1u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(1u128),
                                    match_result: MatchResult::Team1,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(2u128),
                                    match_result: MatchResult::Team1,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(2u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(3u128),
                                    match_result: MatchResult::Draw,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(4u128),
                                    match_result: MatchResult::Draw,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(3u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(5u128),
                                    match_result: MatchResult::Draw,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(6u128),
                                    match_result: MatchResult::Draw,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(4u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(7u128),
                                    match_result: MatchResult::Draw,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(8u128),
                                    match_result: MatchResult::Draw,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(5u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(9u128),
                                    match_result: MatchResult::Draw,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(10u128),
                                    match_result: MatchResult::Draw,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
            ],
        },
        &[],
    );
    assert!(result.is_ok());

    // Check escrow balance was distributed after all matches were processed
    // Distribution was 1st - 70%, 2nd - 20%, 3rd - 10%
    // 1st place is tied between 2 and 2nd place is 1 person - 3rd place's 10% will then be split between 1st and 2nd
    // The new distribution will become 1st - 75% split between 2 and 2nd - 25% for one person
    let balances: Vec<MemberBalanceUnchecked> = context
        .app
        .wrap()
        .query_wasm_smart(
            competition.escrow.as_ref().unwrap(),
            &arena_interface::escrow::QueryMsg::Balances {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(
        balances,
        vec![
            MemberBalanceUnchecked {
                addr: users[0].to_string(),
                balance: BalanceUnchecked {
                    native: coins(6375, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            },
            MemberBalanceUnchecked {
                addr: users[2].to_string(),
                balance: BalanceUnchecked {
                    native: coins(4250, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            },
            MemberBalanceUnchecked {
                addr: users[1].to_string(),
                balance: BalanceUnchecked {
                    native: coins(6375, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            }
        ]
    );

    // Let's test a 1st, 2nd, and 3rd place tie
    let result = create_competition(
        &mut context,
        Expiration::AtHeight(starting_height + 100),
        users[0..4] // Only 4 members this time
            .iter()
            .map(|x| Member {
                addr: x.to_string(),
                weight: 1u64,
            })
            .collect(),
        Some(vec![
            MemberBalanceUnchecked {
                addr: users[0].to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
            MemberBalanceUnchecked {
                addr: users[1].to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
        ]),
        vec![
            Decimal::from_ratio(70u128, 100u128),
            Decimal::from_ratio(20u128, 100u128),
            Decimal::from_ratio(10u128, 100u128),
        ],
        None,
    );
    assert!(result.is_ok());
    let competition_id = get_id_from_competition_creation_result(result);

    // Get the competition
    let competition: LeagueResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::Competition { competition_id },
        )
        .unwrap();

    // Fund escrow
    context
        .app
        .execute_contract(
            users[0].clone(),
            competition.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();
    context
        .app
        .execute_contract(
            users[1].clone(),
            competition.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();

    context.app.update_block(|x| x.height += 10);
    // Process all matches
    let result = context.app.execute_contract(
        admin.clone(),
        context.core.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(1u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(1u128),
                                    match_result: MatchResult::Team1,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(2u128),
                                    match_result: MatchResult::Team1,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(2u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(3u128),
                                    match_result: MatchResult::Team1,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(4u128),
                                    match_result: MatchResult::Team2,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(3u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(5u128),
                                    match_result: MatchResult::Team1,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(6u128),
                                    match_result: MatchResult::Draw,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
            ],
        },
        &[],
    );
    assert!(result.is_ok());

    // Check escrow balance was distributed after all matches were processed
    // Distribution was 1st - 70%, 2nd - 20%, 3rd - 10%
    // 1st place is 1 person, 2nd place is 1 person, and 3rd place is 2 people
    let balances: Vec<MemberBalanceUnchecked> = context
        .app
        .wrap()
        .query_wasm_smart(
            competition.escrow.as_ref().unwrap(),
            &arena_interface::escrow::QueryMsg::Balances {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(
        balances,
        vec![
            MemberBalanceUnchecked {
                addr: users[3].to_string(),
                balance: BalanceUnchecked {
                    native: coins(850, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            },
            MemberBalanceUnchecked {
                addr: users[0].to_string(),
                balance: BalanceUnchecked {
                    native: coins(11900, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            },
            MemberBalanceUnchecked {
                addr: users[2].to_string(),
                balance: BalanceUnchecked {
                    native: coins(850, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            },
            MemberBalanceUnchecked {
                addr: users[1].to_string(),
                balance: BalanceUnchecked {
                    native: coins(3400, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            }
        ]
    );

    // Let's test a 1st with 2nd place tie - 3 members out of 3 distribution
    let result = create_competition(
        &mut context,
        Expiration::AtHeight(starting_height + 100),
        users[0..4] // Only 4 members this time
            .iter()
            .map(|x| Member {
                addr: x.to_string(),
                weight: 1u64,
            })
            .collect(),
        Some(vec![
            MemberBalanceUnchecked {
                addr: users[0].to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
            MemberBalanceUnchecked {
                addr: users[1].to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
        ]),
        vec![
            Decimal::from_ratio(70u128, 100u128),
            Decimal::from_ratio(20u128, 100u128),
            Decimal::from_ratio(10u128, 100u128),
        ],
        None,
    );
    assert!(result.is_ok());
    let competition_id = get_id_from_competition_creation_result(result);

    // Get the competition
    let competition: LeagueResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::Competition { competition_id },
        )
        .unwrap();

    // Fund escrow
    context
        .app
        .execute_contract(
            users[0].clone(),
            competition.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();
    context
        .app
        .execute_contract(
            users[1].clone(),
            competition.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();

    context.app.update_block(|x| x.height += 10);
    // Process all matches
    let result = context.app.execute_contract(
        admin.clone(),
        context.core.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(1u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(1u128),
                                    match_result: MatchResult::Team1,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(2u128),
                                    match_result: MatchResult::Team1,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(2u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(3u128),
                                    match_result: MatchResult::Team2,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(4u128),
                                    match_result: MatchResult::Team2,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(3u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(5u128),
                                    match_result: MatchResult::Draw,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(6u128),
                                    match_result: MatchResult::Draw,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
            ],
        },
        &[],
    );
    assert!(result.is_ok());

    // Check escrow balance was distributed after all matches were processed
    // Distribution was 1st - 70%, 2nd - 20%, 3rd - 10%
    // 1st place is 1 person and 2nd place is 2 people so 3rd's 10% is redistributed up
    let balances: Vec<MemberBalanceUnchecked> = context
        .app
        .wrap()
        .query_wasm_smart(
            competition.escrow.as_ref().unwrap(),
            &arena_interface::escrow::QueryMsg::Balances {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(
        balances,
        vec![
            MemberBalanceUnchecked {
                addr: users[3].to_string(),
                balance: BalanceUnchecked {
                    native: coins(2125, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            },
            MemberBalanceUnchecked {
                addr: users[0].to_string(),
                balance: BalanceUnchecked {
                    native: coins(2125, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            },
            MemberBalanceUnchecked {
                addr: users[1].to_string(),
                balance: BalanceUnchecked {
                    native: coins(12750, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            }
        ]
    );
}

#[test]
fn test_additional_layered_fees() {
    let mut app = get_app();
    let users = [
        app.api().addr_make("user1"),
        app.api().addr_make("user2"),
        app.api().addr_make("user3"),
        app.api().addr_make("user4"),
        app.api().addr_make("user5"),
    ];
    let admin = app.api().addr_make(ADMIN);
    let wager_amount_uint128 = Uint128::from(10_000u128);
    let wager_amount = format!("{}{}", wager_amount_uint128, "juno");

    set_balances(
        &mut app,
        vec![
            (users[0].clone(), Coins::from_str("100000juno").unwrap()),
            (users[1].clone(), Coins::from_str("100000juno").unwrap()),
        ],
    );

    let core_context = setup_core_context(
        &mut app,
        vec![Member {
            addr: admin.to_string(),
            weight: 1u64,
        }],
    );
    let league_context = setup_league_context(&mut app, &core_context);
    let mut context = Context {
        app,
        core: core_context,
        league: league_context,
    };

    // Define additional layered fees
    let additional_layered_fees = Some(vec![
        FeeInformation {
            tax: Decimal::from_ratio(5u128, 100u128),
            receiver: users[2].to_string(),
            cw20_msg: None,
            cw721_msg: None,
        },
        FeeInformation {
            tax: Decimal::from_ratio(5u128, 100u128),
            receiver: users[3].to_string(),
            cw20_msg: None,
            cw721_msg: None,
        },
    ]);

    // Create competition
    let starting_height = context.app.block_info().height;
    let result = create_competition(
        &mut context,
        Expiration::AtHeight(starting_height + 100),
        users
            .iter()
            .map(|x| Member {
                addr: x.to_string(),
                weight: 1u64,
            })
            .collect(),
        Some(vec![
            MemberBalanceUnchecked {
                addr: users[0].to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
            MemberBalanceUnchecked {
                addr: users[1].to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
        ]),
        vec![
            Decimal::from_ratio(70u128, 100u128),
            Decimal::from_ratio(20u128, 100u128),
            Decimal::from_ratio(10u128, 100u128),
        ],
        additional_layered_fees,
    );
    assert!(result.is_ok());
    let competition_id = get_id_from_competition_creation_result(result);

    // Get the competition
    let competition: LeagueResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::Competition { competition_id },
        )
        .unwrap();

    // Fund escrow
    context
        .app
        .execute_contract(
            users[0].clone(),
            competition.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();
    context
        .app
        .execute_contract(
            users[1].clone(),
            competition.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();

    context.app.update_block(|x| x.height += 10);
    // Process all matches
    let result = context.app.execute_contract(
        admin.clone(),
        context.core.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(1u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(1u128),
                                    match_result: MatchResult::Draw,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(2u128),
                                    match_result: MatchResult::Draw,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(2u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(3u128),
                                    match_result: MatchResult::Draw,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(4u128),
                                    match_result: MatchResult::Draw,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(3u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(5u128),
                                    match_result: MatchResult::Draw,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(6u128),
                                    match_result: MatchResult::Draw,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(4u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(7u128),
                                    match_result: MatchResult::Draw,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(8u128),
                                    match_result: MatchResult::Draw,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
                WasmMsg::Execute {
                    contract_addr: context.league.league_module_addr.to_string(),
                    msg: to_json_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::ProcessMatch {
                            league_id: competition_id,
                            round_number: Uint64::from(5u64),
                            match_results: vec![
                                MatchResultMsg {
                                    match_number: Uint128::from(9u128),
                                    match_result: MatchResult::Draw,
                                },
                                MatchResultMsg {
                                    match_number: Uint128::from(10u128),
                                    match_result: MatchResult::Draw,
                                },
                            ],
                        },
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
            ],
        },
        &[],
    );
    assert!(result.is_ok());

    // Check escrow balance was distributed after all matches were processed
    // Distribution was 1st - 70%, 2nd - 20%, 3rd - 10%, but all members tied so it's 20% for everyone
    // Additional layered fees were applied users[2] - 5% then users[3] - 5%
    let balances: Vec<MemberBalanceUnchecked> = context
        .app
        .wrap()
        .query_wasm_smart(
            competition.escrow.as_ref().unwrap(),
            &arena_interface::escrow::QueryMsg::Balances {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(
        balances,
        vec![
            MemberBalanceUnchecked {
                addr: users[3].to_string(),
                balance: BalanceUnchecked {
                    native: coins(3071, "juno"), // This one had the remainders added to it 4 + 3068
                    cw20: vec![],
                    cw721: vec![]
                }
            },
            MemberBalanceUnchecked {
                addr: users[0].to_string(),
                balance: BalanceUnchecked {
                    native: coins(3068, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            },
            MemberBalanceUnchecked {
                addr: users[4].to_string(),
                balance: BalanceUnchecked {
                    native: coins(3068, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            },
            MemberBalanceUnchecked {
                addr: users[2].to_string(),
                balance: BalanceUnchecked {
                    native: coins(3068, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            },
            MemberBalanceUnchecked {
                addr: users[1].to_string(),
                balance: BalanceUnchecked {
                    native: coins(3068, "juno"),
                    cw20: vec![],
                    cw721: vec![]
                }
            }
        ]
    );

    // Let's check that the 5% layered fee was automatically sent
    // 20000 * .85 * .05
    let balance = context
        .app
        .wrap()
        .query_balance(users[2].to_string(), "juno")
        .unwrap();
    assert_eq!(balance.amount, Uint128::from(850u128));

    // Let's check that the 5% layered fee was automatically sent
    // 20000 * .85 * .95 * .05
    let balance = context
        .app
        .wrap()
        .query_balance(users[3].to_string(), "juno")
        .unwrap();
    assert_eq!(balance.amount, Uint128::from(807u128));
}
