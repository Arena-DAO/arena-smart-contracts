use std::str::FromStr;

use arena_league_module::msg::{
    CompetitionInstantiateExt, ExecuteMsg, InstantiateMsg, LeagueResponse, QueryMsg,
};
use cosmwasm_std::{to_json_binary, Addr, Coin, Coins, Empty, Uint128, Uint64, WasmMsg};
use cw4::Member;
use cw_balance::MemberBalance;
use cw_multi_test::{next_block, App, Executor};
use cw_utils::{Duration, Expiration};
use dao_interface::state::ModuleInstantiateInfo;

use crate::tests::core::{get_attr_value, setup_core_context, ADMIN};

use super::core::CoreContext;

struct Context {
    app: App,
    core: CoreContext,
    league: LeagueContext,
}

pub struct LeagueContext {
    pub league_module_addr: Addr,
    pub escrow_id: u64,
}

fn setup_app(balances: Vec<(Addr, Coins)>) -> App {
    App::new(|router, _, storage| {
        for balance in balances {
            router
                .bank
                .init_balance(storage, &balance.0, balance.1.into_vec())
                .unwrap();
        }
    })
}

fn setup_league_context(app: &mut App, core_context: &CoreContext) -> LeagueContext {
    let league_module_id = app.store_code(arena_testing::contracts::arena_league_module_contract());
    let escrow_id = app.store_code(arena_testing::contracts::arena_dao_escrow_contract());

    // Attach the arena-wager-module to the arena-core
    let result = app.execute_contract(
        Addr::unchecked(ADMIN),
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
    dues: Option<Vec<MemberBalance>>,
    round_duration: Duration,
) -> Uint128 {
    let teams: Vec<String> = members.iter().map(|x| x.addr.to_string()).collect();

    let result = context.app.execute_contract(
        Addr::unchecked(ADMIN),
        context.league.league_module_addr.clone(), // errors out bc dao not set
        &ExecuteMsg::CreateCompetition {
            competition_dao: ModuleInstantiateInfo {
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
                wager_dao: ModuleInstantiateInfo {
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
                        vec![],
                    ))
                    .unwrap(),
                    admin: None,
                    label: "Wager DAO".to_string(),
                },
            },
        },
        &[],
    );
    assert!(result.is_ok());

    let id = get_attr_value(&result.unwrap(), "id");
    assert!(id.is_some());

    let result = Uint128::from_str(&id.unwrap());
    assert!(result.is_ok());

    result.unwrap()
}

#[test]
fn test_create_competition() {
    let users = vec![
        Addr::unchecked("user1"),
        Addr::unchecked("user2"),
        Addr::unchecked("user3"),
        Addr::unchecked("user4"),
        Addr::unchecked("user5"),
    ];
    let wager_amount_uint128 = Uint128::from(10_000u128);
    let wager_amount = format!("{}{}", wager_amount_uint128, "juno");

    let mut app = setup_app(vec![
        (users[0].clone(), Coins::from_str(&wager_amount).unwrap()),
        (users[1].clone(), Coins::from_str(&wager_amount).unwrap()),
    ]);
    let core_context = setup_core_context(
        &mut app,
        vec![Member {
            addr: ADMIN.to_string(),
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
            MemberBalance {
                addr: users[0].to_string(),
                balance: cw_balance::Balance {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
            MemberBalance {
                addr: users[1].to_string(),
                balance: cw_balance::Balance {
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
                id: competition1_id,
            },
        )
        .unwrap();
    assert_eq!(competition1.extension.rounds, Uint64::from(5u64));
}
