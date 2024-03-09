use std::str::FromStr;

use arena_league_module::{
    msg::{
        CompetitionInstantiateExt, ExecuteExt, ExecuteMsg, InstantiateMsg, LeagueResponse,
        MatchResult, MemberPoints, QueryExt, QueryMsg,
    },
    state::{Match, Result, RoundResponse, TournamentExt},
};
use cosmwasm_std::{to_json_binary, Addr, Coin, Coins, Decimal, Uint128, Uint64, WasmMsg};
use cw4::Member;
use cw_balance::MemberBalanceUnchecked;
use cw_competition::msg::ModuleInfo;
use cw_multi_test::{addons::MockApiBech32, next_block, App, BankKeeper, Executor};
use cw_utils::{Duration, Expiration};
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

    // Attach the arena-wager-module to the arena-core
    let result = app.execute_contract(
        app.api().addr_make(ADMIN),
        core_context.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_context.arena_core_addr.to_string(),
                funds: vec![],
                msg: to_json_binary(&arena_core_interface::msg::ExecuteMsg::Extension {
                    msg: arena_core_interface::msg::ExecuteExt::UpdateCompetitionModules {
                        to_add: vec![ModuleInstantiateInfo {
                            code_id: league_module_id,
                            msg: to_json_binary(&InstantiateMsg {
                                key: "Leagues".to_string(),
                                description: "This is a description".to_string(),
                                extension: TournamentExt {
                                    tax_cw20_msg: None,
                                    tax_cw721_msg: None,
                                    remainder_addr: core_context.dao_addr.to_string(),
                                },
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

    // Get the wager module addr from the response
    let maybe_val = get_attr_value(result.as_ref().unwrap(), "competition_module_addr");
    assert!(maybe_val.is_some());
    let league_module_addr = Addr::unchecked(maybe_val.unwrap());

    LeagueContext {
        league_module_addr,
        escrow_id,
    }
}

fn create_competition(
    context: &mut Context,
    expiration: Expiration,
    members: Vec<cw4::Member>,
    dues: Option<Vec<MemberBalanceUnchecked>>,
    round_duration: Duration,
) -> Uint128 {
    let teams: Vec<String> = members.iter().map(|x| x.addr.to_string()).collect();

    let result = context.app.execute_contract(
        context.app.api().addr_make(ADMIN),
        context.league.league_module_addr.clone(), // errors out bc dao not set
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
            escrow: dues.map(|x| ModuleInstantiateInfo {
                code_id: context.league.escrow_id,
                msg: to_json_binary(&arena_escrow::msg::InstantiateMsg { dues: x }).unwrap(),
                admin: None,
                label: "Escrow".to_owned(),
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
            instantiate_extension: CompetitionInstantiateExt {
                teams,
                round_duration,
                match_win_points: Uint128::from(3u128),
                match_draw_points: Uint128::one(),
                match_lose_points: Uint128::zero(),
                distribution: vec![
                    Decimal::from_ratio(70u128, 100u128),
                    Decimal::from_ratio(20u128, 100u128),
                    Decimal::from_ratio(10u128, 100u128),
                ],
            },
        },
        &[],
    );
    assert!(result.is_ok());

    let id = get_attr_value(&result.unwrap(), "competition_id");
    assert!(id.is_some());

    let result = Uint128::from_str(&id.unwrap());
    assert!(result.is_ok());

    result.unwrap()
}

#[test]
fn test_create_competition() {
    let mut app = get_app();
    let users = vec![
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

    // Create competiton
    let starting_height = context.app.block_info().height;
    let competition1_id = create_competition(
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
        Duration::Height(10u64),
    );

    // Get competition1
    let competition1: LeagueResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::Competition {
                competition_id: competition1_id,
            },
        )
        .unwrap();
    assert_eq!(competition1.extension.rounds, Uint64::from(5u64));

    // Get round 1
    let round1: RoundResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::QueryExtension {
                msg: QueryExt::Round {
                    league_id: competition1_id,
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
                    team_1: users[2].clone(),
                    team_2: users[3].clone(),
                    result: None
                },
                Match {
                    match_number: Uint128::one(),
                    team_1: users[1].clone(),
                    team_2: users[4].clone(),
                    result: None
                }
            ],
            expiration: Expiration::AtHeight(starting_height + 10u64),
        },
    );

    // Get round 2
    let round1: RoundResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.league.league_module_addr.clone(),
            &QueryMsg::QueryExtension {
                msg: QueryExt::Round {
                    league_id: competition1_id,
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
            expiration: Expiration::AtHeight(starting_height + 20u64),
        },
    );

    context.app.update_block(|x| x.height += 10);
    // Process 1st round of matches
    let result = context.app.execute_contract(
        admin.clone(),
        context.core.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: context.league.league_module_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::Extension {
                    msg: ExecuteExt::ProcessMatch {
                        league_id: competition1_id,
                        round_number: Uint64::one(),
                        match_results: vec![
                            MatchResult {
                                match_number: Uint128::one(),
                                result: Some(Result::Team1),
                            },
                            MatchResult {
                                match_number: Uint128::from(2u128),
                                result: Some(Result::Draw),
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
                msg: QueryExt::Leaderboard {
                    league_id: competition1_id,
                    round: None,
                },
            },
        )
        .unwrap();

    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[1]).unwrap(),
        MemberPoints {
            member: users[1].clone(),
            points: Uint128::from(3u128),
            matches_played: Uint64::one()
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[2]).unwrap(),
        MemberPoints {
            member: users[2].clone(),
            points: Uint128::one(),
            matches_played: Uint64::one()
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[3]).unwrap(),
        MemberPoints {
            member: users[3].clone(),
            points: Uint128::one(),
            matches_played: Uint64::one()
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[4]).unwrap(),
        MemberPoints {
            member: users[4].clone(),
            points: Uint128::zero(),
            matches_played: Uint64::one()
        }
    );

    context.app.update_block(|x| x.height += 10);
    // Process 2nd round of matches
    let result = context.app.execute_contract(
        admin.clone(),
        context.core.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: context.league.league_module_addr.to_string(),
                msg: to_json_binary(&ExecuteMsg::Extension {
                    msg: ExecuteExt::ProcessMatch {
                        league_id: competition1_id,
                        round_number: Uint64::from(2u64),
                        match_results: vec![
                            MatchResult {
                                match_number: Uint128::from(3u128),
                                result: Some(Result::Team1),
                            },
                            MatchResult {
                                match_number: Uint128::from(4u128),
                                result: Some(Result::Team1),
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
                msg: QueryExt::Leaderboard {
                    league_id: competition1_id,
                    round: None,
                },
            },
        )
        .unwrap();

    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[0]).unwrap(),
        MemberPoints {
            member: users[0].clone(),
            points: Uint128::from(3u128),
            matches_played: Uint64::one()
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[1]).unwrap(),
        MemberPoints {
            member: users[1].clone(),
            points: Uint128::from(6u128),
            matches_played: Uint64::from(2u64)
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[2]).unwrap(),
        MemberPoints {
            member: users[2].clone(),
            points: Uint128::one(),
            matches_played: Uint64::from(2u64)
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[3]).unwrap(),
        MemberPoints {
            member: users[3].clone(),
            points: Uint128::one(),
            matches_played: Uint64::one()
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[4]).unwrap(),
        MemberPoints {
            member: users[4].clone(),
            points: Uint128::zero(),
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
                        league_id: competition1_id,
                        round_number: Uint64::from(3u64),
                        match_results: vec![
                            MatchResult {
                                match_number: Uint128::from(5u128),
                                result: Some(Result::Team1),
                            },
                            MatchResult {
                                match_number: Uint128::from(6u128),
                                result: Some(Result::Team1),
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
                msg: QueryExt::Leaderboard {
                    league_id: competition1_id,
                    round: None,
                },
            },
        )
        .unwrap();

    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[0]).unwrap(),
        MemberPoints {
            member: users[0].clone(),
            points: Uint128::from(6u128),
            matches_played: Uint64::from(2u64)
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[1]).unwrap(),
        MemberPoints {
            member: users[1].clone(),
            points: Uint128::from(6u128),
            matches_played: Uint64::from(2u64)
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[2]).unwrap(),
        MemberPoints {
            member: users[2].clone(),
            points: Uint128::one(),
            matches_played: Uint64::from(3u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[3]).unwrap(),
        MemberPoints {
            member: users[3].clone(),
            points: Uint128::one(),
            matches_played: Uint64::from(2u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[4]).unwrap(),
        MemberPoints {
            member: users[4].clone(),
            points: Uint128::from(3u128),
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
                        league_id: competition1_id,
                        round_number: Uint64::from(4u64),
                        match_results: vec![
                            MatchResult {
                                match_number: Uint128::from(7u128),
                                result: Some(Result::Team1),
                            },
                            MatchResult {
                                match_number: Uint128::from(8u128),
                                result: Some(Result::Team1),
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
                msg: QueryExt::Leaderboard {
                    league_id: competition1_id,
                    round: None,
                },
            },
        )
        .unwrap();

    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[0]).unwrap(),
        MemberPoints {
            member: users[0].clone(),
            points: Uint128::from(9u128),
            matches_played: Uint64::from(3u64)
        }
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[1]).unwrap(),
        MemberPoints {
            member: users[1].clone(),
            points: Uint128::from(6u128),
            matches_played: Uint64::from(3u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[2]).unwrap(),
        MemberPoints {
            member: users[2].clone(),
            points: Uint128::one(),
            matches_played: Uint64::from(4u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[3]).unwrap(),
        MemberPoints {
            member: users[3].clone(),
            points: Uint128::from(4u128),
            matches_played: Uint64::from(3u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[4]).unwrap(),
        MemberPoints {
            member: users[4].clone(),
            points: Uint128::from(3u128),
            matches_played: Uint64::from(3u64)
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
                        league_id: competition1_id,
                        round_number: Uint64::from(5u64),
                        match_results: vec![
                            MatchResult {
                                match_number: Uint128::from(9u128),
                                result: Some(Result::Team1),
                            },
                            MatchResult {
                                match_number: Uint128::from(10u128),
                                result: Some(Result::Team1),
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
                msg: QueryExt::Leaderboard {
                    league_id: competition1_id,
                    round: None,
                },
            },
        )
        .unwrap();

    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[0]).unwrap(),
        MemberPoints {
            member: users[0].clone(),
            points: Uint128::from(12u128),
            matches_played: Uint64::from(4u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[1]).unwrap(),
        MemberPoints {
            member: users[1].clone(),
            points: Uint128::from(6u128),
            matches_played: Uint64::from(4u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[2]).unwrap(),
        MemberPoints {
            member: users[2].clone(),
            points: Uint128::one(),
            matches_played: Uint64::from(4u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[3]).unwrap(),
        MemberPoints {
            member: users[3].clone(),
            points: Uint128::from(7u128),
            matches_played: Uint64::from(4u64)
        },
    );
    assert_eq!(
        *leaderboard.iter().find(|x| x.member == users[4]).unwrap(),
        MemberPoints {
            member: users[4].clone(),
            points: Uint128::from(3u128),
            matches_played: Uint64::from(4u64)
        }
    );
}
