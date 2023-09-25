use std::str::FromStr;

use arena_wager_module::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, WagerResponse};
use cosmwasm_std::{to_binary, Addr, Coin, Coins, Empty, Uint128, WasmMsg};
use cw4::Member;
use cw_balance::MemberBalance;
use cw_competition::state::CompetitionStatus;
use cw_multi_test::{next_block, App, Executor};
use cw_utils::Expiration;
use dao_interface::state::{ModuleInstantiateInfo, ProposalModule};

use crate::tests::core::{get_attr_value, setup_core_context, ADMIN};

use super::core::CoreContext;

struct Context {
    app: App,
    core: CoreContext,
    wager: WagerContext,
}

struct WagerContext {
    wager_module_addr: Addr,
    escrow_id: u64,
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

fn setup_wager_context(app: &mut App, core_context: &CoreContext) -> WagerContext {
    let wager_module_id = app.store_code(arena_testing::contracts::arena_wager_module_contract());
    let escrow_id = app.store_code(arena_testing::contracts::arena_dao_escrow_contract());

    // Attach the arena-wager-module to the arena-core
    let result = app.execute_contract(
        Addr::unchecked(ADMIN),
        core_context.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: core_context.arena_core_addr.to_string(),
                funds: vec![],
                msg: to_binary(&arena_core_interface::msg::ExecuteMsg::Extension {
                    msg: arena_core_interface::msg::ExecuteExt::UpdateCompetitionModules {
                        to_add: vec![ModuleInstantiateInfo {
                            code_id: wager_module_id,
                            msg: to_binary(&InstantiateMsg {
                                key: "Wagers".to_string(),
                                description: "This is a description".to_string(),
                                extension: Empty {},
                            })
                            .unwrap(),
                            admin: None,
                            label: "arena-wager-module".to_string(),
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
    let wager_module_addr = Addr::unchecked(maybe_val.unwrap());

    WagerContext {
        wager_module_addr,
        escrow_id,
    }
}

fn create_competition(
    context: &mut Context,
    expiration: Expiration,
    members: Vec<cw4::Member>,
    dues: Option<Vec<MemberBalance>>,
) -> Uint128 {
    let result = context.app.execute_contract(
        Addr::unchecked(ADMIN),
        context.wager.wager_module_addr.clone(), // errors out bc dao not set
        &ExecuteMsg::CreateCompetition {
            competition_dao: ModuleInstantiateInfo {
                code_id: context.core.dao_core_id,
                msg: to_binary(&super::helpers::get_competition_dao_instantiate_msg(
                    context.core.cw4_id,
                    context.core.cw4_voting_module_id,
                    context.core.dao_proposal_multiple_id,
                    dao_proposal_multiple::msg::InstantiateMsg {
                        voting_strategy:
                            dao_voting::multiple_choice::VotingStrategy::SingleChoice {
                                quorum: dao_voting::threshold::PercentageThreshold::Majority {},
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
                code_id: context.wager.escrow_id,
                msg: to_binary(&arena_escrow::msg::InstantiateMsg { dues: x }).unwrap(),
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
            ruleset: None,
            extension: Empty {},
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
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");
    let wager_amount_uint128 = Uint128::from(10_000u128);
    let wager_amount = format!("{}{}", wager_amount_uint128, "juno");

    let mut app = setup_app(vec![
        (user1.clone(), Coins::from_str(&wager_amount).unwrap()),
        (user2.clone(), Coins::from_str(&wager_amount).unwrap()),
    ]);
    let core_context = setup_core_context(
        &mut app,
        vec![Member {
            addr: ADMIN.to_string(),
            weight: 1u64,
        }],
    );
    let wager_context = setup_wager_context(&mut app, &core_context);
    let mut context = Context {
        app,
        core: core_context,
        wager: wager_context,
    };

    // Ensure competition count is zero
    let competition_count: Uint128 = context
        .app
        .wrap()
        .query_wasm_smart(
            context.wager.wager_module_addr.clone(),
            &QueryMsg::CompetitionCount {},
        )
        .unwrap();
    assert_eq!(competition_count, Uint128::zero());

    // Create competiton
    let starting_height = context.app.block_info().height;
    let competition1_id = create_competition(
        &mut context,
        Expiration::AtHeight(starting_height + 10),
        vec![
            cw4::Member {
                addr: user1.to_string(),
                weight: 1u64,
            },
            cw4::Member {
                addr: user2.to_string(),
                weight: 1u64,
            },
        ],
        Some(vec![
            MemberBalance {
                addr: user1.to_string(),
                balance: cw_balance::Balance {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
            MemberBalance {
                addr: user2.to_string(),
                balance: cw_balance::Balance {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
        ]),
    );

    // Ensure competition count is updated
    let competition_count: Uint128 = context
        .app
        .wrap()
        .query_wasm_smart(
            context.wager.wager_module_addr.clone(),
            &QueryMsg::CompetitionCount {},
        )
        .unwrap();
    assert_eq!(competition_count, Uint128::one());

    // Get competition1
    let competition1: WagerResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.wager.wager_module_addr.clone(),
            &QueryMsg::Competition {
                id: competition1_id,
                include_ruleset: Some(false),
            },
        )
        .unwrap();

    // Get competition1 proposal module
    let result = context.app.wrap().query_wasm_smart::<Vec<ProposalModule>>(
        competition1.dao,
        &dao_interface::msg::QueryMsg::ProposalModules {
            start_after: None,
            limit: None,
        },
    );
    assert!(result.is_ok());
    assert!(!result.as_ref().unwrap().is_empty());
    let competition1_proposal_module = result.as_ref().unwrap().first().unwrap();

    // Generate proposals
    context.app.update_block(next_block);
    let result = context.app.execute_contract(
        user1.clone(),
        context.wager.wager_module_addr.clone(),
        &arena_wager_module::msg::ExecuteMsg::GenerateProposals {
            id: competition1_id,
            proposal_module_addr: competition1_proposal_module.address.to_string(),
            proposal_details: arena_core_interface::msg::ProposalDetails {
                title: "Test Title".to_string(),
                description: "Test description".to_string(),
            },
        },
        &[],
    );
    assert!(result.is_ok());

    // Fund escrow
    context
        .app
        .execute_contract(
            user1.clone(),
            competition1.escrow.as_ref().unwrap().clone(),
            &arena_escrow::msg::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();
    context
        .app
        .execute_contract(
            user2.clone(),
            competition1.escrow.as_ref().unwrap().clone(),
            &arena_escrow::msg::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();

    // Ensure competition is active now
    let competition1: WagerResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.wager.wager_module_addr.clone(),
            &QueryMsg::Competition {
                id: competition1_id,
                include_ruleset: Some(false),
            },
        )
        .unwrap();
    assert_eq!(competition1.status, CompetitionStatus::Active);

    // Vote and execute jail
    let result = context.app.execute_contract(
        user1.clone(),
        competition1_proposal_module.address.clone(),
        &dao_proposal_multiple::msg::ExecuteMsg::Vote {
            proposal_id: 1u64,
            vote: dao_voting::multiple_choice::MultipleChoiceVote { option_id: 0u32 },
            rationale: None,
        },
        &[],
    );
    assert!(result.is_ok());

    let result = context.app.execute_contract(
        user2.clone(),
        competition1_proposal_module.address.clone(),
        &dao_proposal_multiple::msg::ExecuteMsg::Vote {
            proposal_id: 1u64,
            vote: dao_voting::multiple_choice::MultipleChoiceVote { option_id: 0u32 },
            rationale: None,
        },
        &[],
    );
    assert!(result.is_ok());

    let result = context.app.execute_contract(
        user1.clone(),
        competition1_proposal_module.address.clone(),
        &dao_proposal_multiple::msg::ExecuteMsg::Execute { proposal_id: 1u64 },
        &[],
    );
    assert!(result.is_ok());

    // Assert correct balances user 1 - 20_000*.85, dao - 20_000*.15
    let balance = context
        .app
        .wrap()
        .query_balance(user1.to_string(), "juno")
        .unwrap();
    assert_eq!(balance.amount, Uint128::from(17_000u128));
    let balance = context
        .app
        .wrap()
        .query_balance(context.core.dao_addr.to_string(), "juno")
        .unwrap();
    assert_eq!(balance.amount, Uint128::from(3_000u128));

    // Assert result is populated
    let competition1: WagerResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.wager.wager_module_addr.clone(),
            &QueryMsg::Competition {
                id: competition1_id,
                include_ruleset: Some(false),
            },
        )
        .unwrap();
    assert!(competition1.result.is_some());
}

#[test]
fn test_create_competition_jailed() {
    let user1 = Addr::unchecked("user1");
    let user2 = Addr::unchecked("user2");
    let wager_amount_uint128 = Uint128::from(10_000u128);
    let wager_amount = format!("{}{}", wager_amount_uint128, "juno");

    let mut app = setup_app(vec![
        (user1.clone(), Coins::from_str(&wager_amount).unwrap()),
        (user2.clone(), Coins::from_str(&wager_amount).unwrap()),
    ]);
    let core_context = setup_core_context(
        &mut app,
        vec![Member {
            addr: ADMIN.to_string(),
            weight: 1u64,
        }],
    );
    let wager_context = setup_wager_context(&mut app, &core_context);
    let mut context = Context {
        app,
        core: core_context,
        wager: wager_context,
    };

    // Create competiton
    let starting_height = context.app.block_info().height;
    let competition1_id = create_competition(
        &mut context,
        Expiration::AtHeight(starting_height + 1),
        vec![
            cw4::Member {
                addr: user1.to_string(),
                weight: 1u64,
            },
            cw4::Member {
                addr: user2.to_string(),
                weight: 1u64,
            },
        ],
        Some(vec![
            MemberBalance {
                addr: user1.to_string(),
                balance: cw_balance::Balance {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
            MemberBalance {
                addr: user2.to_string(),
                balance: cw_balance::Balance {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
        ]),
    );

    // Ensure not expired
    let competition1: WagerResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.wager.wager_module_addr.clone(),
            &QueryMsg::Competition {
                id: competition1_id,
                include_ruleset: Some(false),
            },
        )
        .unwrap();
    assert!(!competition1.is_expired);

    // Ensure expired after updating block
    context.app.update_block(next_block);

    let competition1: WagerResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.wager.wager_module_addr.clone(),
            &QueryMsg::Competition {
                id: competition1_id,
                include_ruleset: Some(false),
            },
        )
        .unwrap();
    assert!(competition1.is_expired);

    // Still cannot jail competition - not active
    let result = context.app.execute_contract(
        user1.clone(),
        context.wager.wager_module_addr.clone(),
        &ExecuteMsg::JailCompetition {
            id: competition1_id,
            proposal_details: arena_core_interface::msg::ProposalDetails {
                title: "Title".to_string(),
                description: "Description".to_string(),
            },
        },
        &[],
    );
    assert!(result.is_err());

    // Fund escrow
    context
        .app
        .execute_contract(
            user1.clone(),
            competition1.escrow.as_ref().unwrap().clone(),
            &arena_escrow::msg::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();
    context
        .app
        .execute_contract(
            user2.clone(),
            competition1.escrow.as_ref().unwrap().clone(),
            &arena_escrow::msg::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();

    // Ensure competition is active now
    let competition1: WagerResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.wager.wager_module_addr.clone(),
            &QueryMsg::Competition {
                id: competition1_id,
                include_ruleset: Some(false),
            },
        )
        .unwrap();
    assert_eq!(competition1.status, CompetitionStatus::Active);

    // Cannot jail wager - unauthorized
    let result = context.app.execute_contract(
        Addr::unchecked("random"),
        context.wager.wager_module_addr.clone(),
        &ExecuteMsg::JailCompetition {
            id: competition1_id,
            proposal_details: arena_core_interface::msg::ProposalDetails {
                title: "Title".to_string(),
                description: "Description".to_string(),
            },
        },
        &[],
    );
    assert!(result.is_err());

    // Can jail wager
    let result = context.app.execute_contract(
        user1.clone(),
        context.wager.wager_module_addr.clone(),
        &ExecuteMsg::JailCompetition {
            id: competition1_id,
            proposal_details: arena_core_interface::msg::ProposalDetails {
                title: "Title".to_string(),
                description: "Description".to_string(),
            },
        },
        &[],
    );
    assert!(result.is_ok());

    // Check valid jailed state
    let competition1: WagerResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.wager.wager_module_addr.clone(),
            &QueryMsg::Competition {
                id: competition1_id,
                include_ruleset: Some(false),
            },
        )
        .unwrap();
    assert_eq!(competition1.status, CompetitionStatus::Jailed);

    // Cannot jail again
    let result = context.app.execute_contract(
        user1.clone(),
        context.wager.wager_module_addr.clone(),
        &ExecuteMsg::JailCompetition {
            id: competition1_id,
            proposal_details: arena_core_interface::msg::ProposalDetails {
                title: "Title".to_string(),
                description: "Description".to_string(),
            },
        },
        &[],
    );
    assert!(result.is_err());

    // Vote and execute jail
    let result = context.app.execute_contract(
        Addr::unchecked(ADMIN),
        context.core.proposal_module_addr.clone(),
        &dao_proposal_multiple::msg::ExecuteMsg::Vote {
            proposal_id: 1u64,
            vote: dao_voting::multiple_choice::MultipleChoiceVote { option_id: 0u32 },
            rationale: None,
        },
        &[],
    );
    assert!(result.is_ok());

    let result = context.app.execute_contract(
        Addr::unchecked(ADMIN),
        context.core.proposal_module_addr.clone(),
        &dao_proposal_multiple::msg::ExecuteMsg::Execute { proposal_id: 1u64 },
        &[],
    );
    assert!(result.is_ok());

    // Assert correct balances user 1 - 20_000*.85, dao - 20_000*.15
    let balance = context
        .app
        .wrap()
        .query_balance(user1.to_string(), "juno")
        .unwrap();
    assert_eq!(balance.amount, Uint128::from(17_000u128));
    let balance = context
        .app
        .wrap()
        .query_balance(context.core.dao_addr.to_string(), "juno")
        .unwrap();
    assert_eq!(balance.amount, Uint128::from(3_000u128));
}
