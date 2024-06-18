use std::str::FromStr;

use arena_interface::competition::{
    msg::{EscrowInstantiateInfo, ModuleInfo},
    state::{CompetitionListItemResponse, CompetitionStatus},
};
use arena_interface::core::{
    CompetitionModuleQuery, CompetitionModuleResponse, ProposeMessage, QueryExt,
};
use arena_wager_module::msg::{
    ExecuteMsg, InstantiateMsg, QueryMsg, WagerInstantiateExt, WagerResponse,
};
use cosmwasm_std::{
    to_json_binary, Addr, Coin, Coins, CosmosMsg, Decimal, Empty, Uint128, WasmMsg,
};
use cw4::Member;
use cw_balance::{
    BalanceVerified, Distribution, MemberBalanceChecked, MemberBalanceUnchecked, MemberPercentage,
};
use cw_multi_test::{next_block, App, BankKeeper, Executor, MockApiBech32};
use cw_utils::Expiration;
use dao_interface::state::{ModuleInstantiateInfo, ProposalModule};
use dao_voting::proposal::SingleChoiceProposeMsg;

use crate::tests::{
    app::{get_app, set_balances},
    core::{get_attr_value, setup_core_context, ADMIN},
};

use super::core::CoreContext;

struct Context {
    app: App<BankKeeper, MockApiBech32>,
    core: CoreContext,
    wager: WagerContext,
}

pub struct WagerContext {
    pub wager_module_addr: Addr,
    pub escrow_id: u64,
    pub wagers_key: String,
}

pub fn setup_wager_context(
    app: &mut App<BankKeeper, MockApiBech32>,
    core_context: &CoreContext,
) -> WagerContext {
    let wager_module_id = app.store_code(arena_testing::contracts::arena_wager_module_contract());
    let escrow_id = app.store_code(arena_testing::contracts::arena_dao_escrow_contract());
    let wagers_key = "Wagers".to_string();

    // Attach the arena-wager-module to the arena-core
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
                            code_id: wager_module_id,
                            msg: to_json_binary(&InstantiateMsg {
                                key: wagers_key.clone(),
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

    // Get the wager module
    let wager_module = app
        .wrap()
        .query_wasm_smart::<CompetitionModuleResponse<Addr>>(
            core_context.arena_core_addr.clone(),
            &arena_interface::core::QueryMsg::QueryExtension {
                msg: QueryExt::CompetitionModule {
                    query: CompetitionModuleQuery::Key(wagers_key.clone(), None),
                },
            },
        )
        .unwrap();

    WagerContext {
        wager_module_addr: wager_module.addr,
        escrow_id,
        wagers_key,
    }
}

fn create_competition(
    context: &mut Context,
    expiration: Expiration,
    members: Vec<cw4::Member>,
    dues: Option<Vec<MemberBalanceUnchecked>>,
) -> Uint128 {
    let result = context.app.execute_contract(
        context.app.api().addr_make(ADMIN),
        context.wager.wager_module_addr.clone(), // errors out bc dao not set
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
                code_id: context.wager.escrow_id,
                msg: to_json_binary(&arena_escrow::msg::InstantiateMsg {
                    dues: x,
                    should_activate_on_funded: None,
                })
                .unwrap(),
                label: "Escrow".to_owned(),
                additional_layered_fees: None,
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
            instantiate_extension: WagerInstantiateExt {
                registered_members: None,
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
    let user1 = app.api().addr_make("user1");
    let user2 = app.api().addr_make("user2");
    let wager_amount_uint128 = Uint128::from(10_000u128);
    let wager_amount = format!("{}{}", wager_amount_uint128, "juno");
    let admin = app.api().addr_make(ADMIN);

    set_balances(
        &mut app,
        vec![
            (user1.clone(), Coins::from_str(&wager_amount).unwrap()),
            (user2.clone(), Coins::from_str(&wager_amount).unwrap()),
        ],
    );
    let core_context = setup_core_context(
        &mut app,
        vec![Member {
            addr: admin.to_string(),
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

    let starting_height = context.app.block_info().height;

    // Create competition fails from rulesets not existing on core
    let result = context.app.execute_contract(
        admin.clone(),
        context.wager.wager_module_addr.clone(), // errors out bc dao not set
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
                    ))
                    .unwrap(),
                    admin: None,
                    label: "DAO".to_owned(),
                },
            },
            escrow: None,
            name: "This is a competition name".to_string(),
            description: "This is a description".to_string(),
            expiration: Expiration::AtHeight(starting_height + 10),
            rules: vec![
                "Rule 1".to_string(),
                "Rule 2".to_string(),
                "Rule 3".to_string(),
            ],
            rulesets: vec![Uint128::from(9999u128)],
            banner: None,
            should_activate_on_funded: None,
            instantiate_extension: WagerInstantiateExt {
                registered_members: None,
            },
        },
        &[],
    );
    assert!(result.is_err());

    // Create competition
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
            MemberBalanceUnchecked {
                addr: user1.to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
            MemberBalanceUnchecked {
                addr: user2.to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
        ]),
    );

    // Ensure query by competition status works
    let competitions: Vec<CompetitionListItemResponse<Empty>> = context
        .app
        .wrap()
        .query_wasm_smart(
            context.wager.wager_module_addr.clone(),
            &QueryMsg::Competitions {
                start_after: None,
                limit: None,
                filter: Some(
                    arena_interface::competition::msg::CompetitionsFilter::CompetitionStatus {
                        status: CompetitionStatus::Pending,
                    },
                ),
            },
        )
        .unwrap();
    assert_eq!(competitions.len(), 1);

    // Ensure query by competition category works
    let competitions: Vec<CompetitionListItemResponse<Empty>> = context
        .app
        .wrap()
        .query_wasm_smart(
            context.wager.wager_module_addr.clone(),
            &QueryMsg::Competitions {
                start_after: None,
                limit: None,
                filter: Some(
                    arena_interface::competition::msg::CompetitionsFilter::Category {
                        id: Some(Uint128::one()),
                    },
                ),
            },
        )
        .unwrap();

    assert_eq!(competitions.len(), 1);

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
                competition_id: competition1_id,
            },
        )
        .unwrap();

    // Get competition1 proposal module
    let result = context.app.wrap().query_wasm_smart::<Vec<ProposalModule>>(
        competition1.host,
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
        competition1_proposal_module.address.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Propose(SingleChoiceProposeMsg {
            title: "Title".to_string(),
            description: "Description".to_string(),
            msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: context.wager.wager_module_addr.to_string(),
                msg: to_json_binary(&arena_interface::competition::msg::ExecuteBase::<
                    Empty,
                    Empty,
                >::ProcessCompetition {
                    competition_id: competition1_id,
                    distribution: Some(Distribution::<String> {
                        member_percentages: vec![MemberPercentage {
                            addr: user1.to_string(),
                            percentage: Decimal::one(),
                        }],
                        remainder_addr: context.core.dao_addr.to_string(),
                    }),
                })
                .unwrap(),
                funds: vec![],
            })],
            proposer: None,
        }),
        &[],
    );
    assert!(result.is_ok());

    // Fund escrow
    context
        .app
        .execute_contract(
            user1.clone(),
            competition1.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();
    context
        .app
        .execute_contract(
            user2.clone(),
            competition1.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
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
                competition_id: competition1_id,
            },
        )
        .unwrap();
    assert_eq!(competition1.status, CompetitionStatus::Active);

    // Vote and execute
    let result = context.app.execute_contract(
        user1.clone(),
        competition1_proposal_module.address.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Vote {
            proposal_id: 1u64,
            rationale: None,
            vote: dao_voting::voting::Vote::Yes,
        },
        &[],
    );
    assert!(result.is_ok());

    let result = context.app.execute_contract(
        user2.clone(),
        competition1_proposal_module.address.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Vote {
            proposal_id: 1u64,
            vote: dao_voting::voting::Vote::Yes,
            rationale: None,
        },
        &[],
    );
    assert!(result.is_ok());

    let result = context.app.execute_contract(
        user1.clone(),
        competition1_proposal_module.address.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Execute { proposal_id: 1u64 },
        &[],
    );
    assert!(result.is_ok());

    // Claim balances
    let result = context.app.execute_contract(
        user1.clone(),
        competition1.escrow.clone().unwrap(),
        &arena_interface::escrow::ExecuteMsg::Withdraw {
            cw20_msg: None,
            cw721_msg: None,
        },
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
    let result: Option<Distribution<String>> = context
        .app
        .wrap()
        .query_wasm_smart(
            context.wager.wager_module_addr.clone(),
            &QueryMsg::Result {
                competition_id: competition1_id,
            },
        )
        .unwrap();
    assert!(result.is_some());
}

#[test]
fn test_create_competition_jailed() {
    let mut app = get_app();

    let user1 = app.api().addr_make("user1");
    let user2 = app.api().addr_make("user2");
    let admin = app.api().addr_make(ADMIN);
    let wager_amount_uint128 = Uint128::from(10_000u128);
    let wager_amount = format!("{}{}", wager_amount_uint128, "juno");

    set_balances(
        &mut app,
        vec![
            (user1.clone(), Coins::from_str(&wager_amount).unwrap()),
            (user2.clone(), Coins::from_str(&wager_amount).unwrap()),
        ],
    );
    let core_context = setup_core_context(
        &mut app,
        vec![Member {
            addr: admin.to_string(),
            weight: 1u64,
        }],
    );
    let wager_context = setup_wager_context(&mut app, &core_context);
    let mut context = Context {
        app,
        core: core_context,
        wager: wager_context,
    };

    // Create competition
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
            MemberBalanceUnchecked {
                addr: user1.to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
            MemberBalanceUnchecked {
                addr: user2.to_string(),
                balance: cw_balance::BalanceUnchecked {
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
                competition_id: competition1_id,
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
                competition_id: competition1_id,
            },
        )
        .unwrap();
    assert!(competition1.is_expired);

    // Cannot jail - not active
    let propose_message = ProposeMessage {
        competition_id: competition1_id,
        title: "Title".to_string(),
        description: "Description".to_string(),
        distribution: Some(Distribution::<String> {
            member_percentages: vec![MemberPercentage {
                addr: user1.to_string(),
                percentage: Decimal::one(),
            }],
            remainder_addr: context.core.dao_addr.to_string(),
        }),
        additional_layered_fees: None,
    };

    let result = context.app.execute_contract(
        user1.clone(),
        context.wager.wager_module_addr.clone(),
        &ExecuteMsg::JailCompetition {
            propose_message: propose_message.clone(),
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
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();
    context
        .app
        .execute_contract(
            user2.clone(),
            competition1.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
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
                competition_id: competition1_id,
            },
        )
        .unwrap();
    assert_eq!(competition1.status, CompetitionStatus::Active);

    // Cannot jail wager - unauthorized
    let result = context.app.execute_contract(
        context.app.api().addr_make("random"),
        context.wager.wager_module_addr.clone(),
        &ExecuteMsg::JailCompetition {
            propose_message: propose_message.clone(),
        },
        &[],
    );
    assert!(result.is_err());

    // Can jail wager
    let result = context.app.execute_contract(
        user1.clone(),
        context.wager.wager_module_addr.clone(),
        &ExecuteMsg::JailCompetition {
            propose_message: propose_message.clone(),
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
                competition_id: competition1_id,
            },
        )
        .unwrap();
    assert_eq!(competition1.status, CompetitionStatus::Jailed);

    // Can generate jail proposal again
    let result = context.app.execute_contract(
        user1.clone(),
        context.wager.wager_module_addr.clone(),
        &ExecuteMsg::JailCompetition {
            propose_message: propose_message.clone(),
        },
        &[],
    );
    assert!(result.is_ok());

    // Vote and execute jail
    let result = context.app.execute_contract(
        admin.clone(),
        context.core.proposal_module_addr.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Vote {
            proposal_id: 1u64,
            vote: dao_voting::voting::Vote::Yes,
            rationale: None,
        },
        &[],
    );
    assert!(result.is_ok());

    let result = context.app.execute_contract(
        admin.clone(),
        context.core.proposal_module_addr.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Execute { proposal_id: 1u64 },
        &[],
    );
    assert!(result.is_ok());

    // Claim balances
    let result = context.app.execute_contract(
        user1.clone(),
        competition1.escrow.clone().unwrap(),
        &arena_interface::escrow::ExecuteMsg::Withdraw {
            cw20_msg: None,
            cw721_msg: None,
        },
        &[],
    );
    assert!(result.is_ok());

    // Assert correct balances user 1 - 20_000*.85, dao - 20_000*.15
    let balance = context
        .app
        .wrap()
        .query_balance(context.core.dao_addr.to_string(), "juno")
        .unwrap();
    assert_eq!(balance.amount, Uint128::from(3_000u128));
    let balance = context
        .app
        .wrap()
        .query_balance(user1.to_string(), "juno")
        .unwrap();
    assert_eq!(balance.amount, Uint128::from(17_000u128));
}

#[test]
pub fn test_disabling_module() {
    let mut app = get_app();

    let admin = app.api().addr_make(ADMIN);
    let core_context = setup_core_context(
        &mut app,
        vec![Member {
            addr: admin.to_string(),
            weight: 1u64,
        }],
    );
    let wager_context = setup_wager_context(&mut app, &core_context);
    let mut context = Context {
        app,
        core: core_context,
        wager: wager_context,
    };

    // Disable the wager module
    let result = context.app.execute_contract(
        admin.clone(),
        context.core.sudo_proposal_addr.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: context.core.arena_core_addr.to_string(),
                funds: vec![],
                msg: to_json_binary(&arena_interface::core::ExecuteMsg::Extension {
                    msg: arena_interface::core::ExecuteExt::UpdateCompetitionModules {
                        to_add: vec![],
                        to_disable: vec![context.wager.wager_module_addr.to_string()],
                    },
                })
                .unwrap(),
            }
            .into()],
        },
        &[],
    );
    assert!(result.is_ok());

    // Check that the module is disabled
    let competition_module: Option<CompetitionModuleResponse<String>> = context
        .app
        .wrap()
        .query_wasm_smart(
            context.core.arena_core_addr.clone(),
            &arena_interface::core::QueryMsg::QueryExtension {
                msg: QueryExt::CompetitionModule {
                    query: CompetitionModuleQuery::Addr(
                        context.wager.wager_module_addr.to_string(),
                    ),
                },
            },
        )
        .unwrap();
    assert!(competition_module.is_some());
    assert!(!competition_module.unwrap().is_enabled);

    let competition_module: Option<CompetitionModuleResponse<String>> = context
        .app
        .wrap()
        .query_wasm_smart(
            context.core.arena_core_addr.clone(),
            &arena_interface::core::QueryMsg::QueryExtension {
                msg: QueryExt::CompetitionModule {
                    query: CompetitionModuleQuery::Key(context.wager.wagers_key, None),
                },
            },
        )
        .unwrap();
    assert!(competition_module.is_some());
    assert!(!competition_module.unwrap().is_enabled);
}

#[test]
fn test_preset_distribution() {
    let mut app = get_app();
    let user1 = app.api().addr_make("user1");
    let user2 = app.api().addr_make("user2");
    let wager_amount_uint128 = Uint128::from(10_000u128);
    let wager_amount = format!("{}{}", wager_amount_uint128, "juno");
    let admin = app.api().addr_make(ADMIN);

    set_balances(
        &mut app,
        vec![
            (user1.clone(), Coins::from_str(&wager_amount).unwrap()),
            (user2.clone(), Coins::from_str(&wager_amount).unwrap()),
        ],
    );
    let core_context = setup_core_context(
        &mut app,
        vec![Member {
            addr: admin.to_string(),
            weight: 1u64,
        }],
    );
    let wager_context = setup_wager_context(&mut app, &core_context);
    let mut context = Context {
        app,
        core: core_context,
        wager: wager_context,
    };

    let starting_height = context.app.block_info().height;

    // Create competition
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
            MemberBalanceUnchecked {
                addr: user1.to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
            MemberBalanceUnchecked {
                addr: user2.to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
        ]),
    );

    // Get competition1
    let competition1: WagerResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.wager.wager_module_addr.clone(),
            &QueryMsg::Competition {
                competition_id: competition1_id,
            },
        )
        .unwrap();

    // Get competition1 proposal module
    let result = context.app.wrap().query_wasm_smart::<Vec<ProposalModule>>(
        competition1.host,
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
        competition1_proposal_module.address.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Propose(SingleChoiceProposeMsg {
            title: "Title".to_string(),
            description: "Description".to_string(),
            msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: context.wager.wager_module_addr.to_string(),
                msg: to_json_binary(&arena_interface::competition::msg::ExecuteBase::<
                    Empty,
                    Empty,
                >::ProcessCompetition {
                    competition_id: competition1_id,
                    distribution: Some(Distribution::<String> {
                        member_percentages: vec![
                            MemberPercentage::<String> {
                                addr: user1.to_string(),
                                percentage: Decimal::from_ratio(25u128, 100u128),
                            },
                            MemberPercentage::<String> {
                                addr: user2.to_string(),
                                percentage: Decimal::from_ratio(75u128, 100u128),
                            },
                        ],
                        remainder_addr: user1.to_string(),
                    }),
                })
                .unwrap(),
                funds: vec![],
            })],
            proposer: None,
        }),
        &[],
    );
    assert!(result.is_ok());

    // Set distributions .25 to user1 and .75 to user2 in both cases
    context
        .app
        .execute_contract(
            user1.clone(),
            competition1.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::SetDistribution {
                distribution: Some(Distribution::<String> {
                    member_percentages: vec![
                        MemberPercentage::<String> {
                            addr: user1.to_string(),
                            percentage: Decimal::from_ratio(25u128, 100u128),
                        },
                        MemberPercentage::<String> {
                            addr: user2.to_string(),
                            percentage: Decimal::from_ratio(75u128, 100u128),
                        },
                    ],
                    remainder_addr: user1.to_string(),
                }),
            },
            &[],
        )
        .unwrap();
    context
        .app
        .execute_contract(
            user2.clone(),
            competition1.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::SetDistribution {
                distribution: Some(Distribution::<String> {
                    member_percentages: vec![
                        MemberPercentage::<String> {
                            addr: user1.to_string(),
                            percentage: Decimal::from_ratio(50u128, 100u128),
                        },
                        MemberPercentage::<String> {
                            addr: user2.to_string(),
                            percentage: Decimal::from_ratio(50u128, 100u128),
                        },
                    ],
                    remainder_addr: user2.to_string(),
                }),
            },
            &[],
        )
        .unwrap();

    // Fund escrow
    context
        .app
        .execute_contract(
            user1.clone(),
            competition1.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();
    context
        .app
        .execute_contract(
            user2.clone(),
            competition1.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();

    // Trying to update the preset distribution while locked is an error
    let result = context.app.execute_contract(
        user1.clone(),
        competition1.escrow.as_ref().unwrap().clone(),
        &arena_interface::escrow::ExecuteMsg::SetDistribution { distribution: None },
        &[],
    );
    assert!(result.is_err());

    // Vote and execute
    let result = context.app.execute_contract(
        user1.clone(),
        competition1_proposal_module.address.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Vote {
            proposal_id: 1u64,
            rationale: None,
            vote: dao_voting::voting::Vote::Yes,
        },
        &[],
    );
    assert!(result.is_ok());

    let result = context.app.execute_contract(
        user2.clone(),
        competition1_proposal_module.address.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Vote {
            proposal_id: 1u64,
            vote: dao_voting::voting::Vote::Yes,
            rationale: None,
        },
        &[],
    );
    assert!(result.is_ok());

    let result = context.app.execute_contract(
        user1.clone(),
        competition1_proposal_module.address.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Execute { proposal_id: 1u64 },
        &[],
    );
    assert!(result.is_ok());

    // Assert query balances are correct
    /*
       20,000 wager is 17,000 distribution after 15% tax
       result = .25 u1 and .75 u2, so u1 = 4250 and u2 = 12750
       applying preset distributions
        u1 has .25u1 and .75u2 with remainder u1, so u1_1 = 1063 and u2_1 = 3187
        u2 has .5u1 and .5u2, so u1_2 = 6375 and u2_2 = 6375
        u1 = u1_1 + u1_2 = 7438 and u2 = u2_1 + u2_2 = 9562
    */
    let balances: Vec<MemberBalanceChecked> = context
        .app
        .wrap()
        .query_wasm_smart(
            competition1.escrow.clone().unwrap(),
            &arena_interface::escrow::QueryMsg::Balances {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(
        balances[0].balance.native[0].amount,
        Uint128::from(7_438u128)
    );
    assert_eq!(
        balances[1].balance.native[0].amount,
        Uint128::from(9_562u128)
    );
}

#[test]
fn test_competition_draw() {
    let mut app = get_app();
    let user1 = app.api().addr_make("user1");
    let user2 = app.api().addr_make("user2");
    let wager_amount_uint128 = Uint128::from(10_000u128);
    let wager_amount = format!("{}{}", wager_amount_uint128, "juno");
    let admin = app.api().addr_make(ADMIN);

    set_balances(
        &mut app,
        vec![
            (user1.clone(), Coins::from_str(&wager_amount).unwrap()),
            (user2.clone(), Coins::from_str(&wager_amount).unwrap()),
        ],
    );
    let core_context = setup_core_context(
        &mut app,
        vec![Member {
            addr: admin.to_string(),
            weight: 1u64,
        }],
    );
    let wager_context = setup_wager_context(&mut app, &core_context);
    let mut context = Context {
        app,
        core: core_context,
        wager: wager_context,
    };

    let starting_height = context.app.block_info().height;

    // Create competition
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
            MemberBalanceUnchecked {
                addr: user1.to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
            MemberBalanceUnchecked {
                addr: user2.to_string(),
                balance: cw_balance::BalanceUnchecked {
                    native: vec![Coin::from_str(&wager_amount).unwrap()],
                    cw20: vec![],
                    cw721: vec![],
                },
            },
        ]),
    );

    // Get competition1
    let competition1: WagerResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.wager.wager_module_addr.clone(),
            &QueryMsg::Competition {
                competition_id: competition1_id,
            },
        )
        .unwrap();

    // Get competition1 proposal module
    let result = context.app.wrap().query_wasm_smart::<Vec<ProposalModule>>(
        competition1.host,
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
        competition1_proposal_module.address.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Propose(SingleChoiceProposeMsg {
            title: "Title".to_string(),
            description: "Description".to_string(),
            msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: context.wager.wager_module_addr.to_string(),
                msg: to_json_binary(&arena_interface::competition::msg::ExecuteBase::<
                    Empty,
                    Empty,
                >::ProcessCompetition {
                    competition_id: competition1_id,
                    distribution: None,
                })
                .unwrap(),
                funds: vec![],
            })],
            proposer: None,
        }),
        &[],
    );
    assert!(result.is_ok());

    // Fund escrow
    context
        .app
        .execute_contract(
            user1.clone(),
            competition1.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();
    context
        .app
        .execute_contract(
            user2.clone(),
            competition1.escrow.as_ref().unwrap().clone(),
            &arena_interface::escrow::ExecuteMsg::ReceiveNative {},
            &[Coin::from_str(&wager_amount).unwrap()],
        )
        .unwrap();

    // Vote and execute
    let result = context.app.execute_contract(
        user1.clone(),
        competition1_proposal_module.address.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Vote {
            proposal_id: 1u64,
            rationale: None,
            vote: dao_voting::voting::Vote::Yes,
        },
        &[],
    );
    assert!(result.is_ok());

    let result = context.app.execute_contract(
        user2.clone(),
        competition1_proposal_module.address.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Vote {
            proposal_id: 1u64,
            vote: dao_voting::voting::Vote::Yes,
            rationale: None,
        },
        &[],
    );
    assert!(result.is_ok());

    let result = context.app.execute_contract(
        user1.clone(),
        competition1_proposal_module.address.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Execute { proposal_id: 1u64 },
        &[],
    );
    assert!(result.is_ok());

    // Assert query balances are correct
    let balances: Vec<MemberBalanceChecked> = context
        .app
        .wrap()
        .query_wasm_smart(
            competition1.escrow.clone().unwrap(),
            &arena_interface::escrow::QueryMsg::Balances {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(
        balances[0].balance.native[0].amount,
        Uint128::from(8_500u128)
    );
    assert_eq!(
        balances[1].balance.native[0].amount,
        Uint128::from(8_500u128)
    );

    // Assert individual balance query is correct
    let balance: Option<BalanceVerified> = context
        .app
        .wrap()
        .query_wasm_smart(
            competition1.escrow.clone().unwrap(),
            &arena_interface::escrow::QueryMsg::Balance {
                addr: user1.to_string(),
            },
        )
        .unwrap();
    assert!(balance.is_some());
    assert_eq!(balance.unwrap().native[0].amount, Uint128::from(8_500u128));

    // Claim balances
    let result = context.app.execute_contract(
        user1.clone(),
        competition1.escrow.clone().unwrap(),
        &arena_interface::escrow::ExecuteMsg::Withdraw {
            cw20_msg: None,
            cw721_msg: None,
        },
        &[],
    );
    assert!(result.is_ok());
    let result = context.app.execute_contract(
        user2.clone(),
        competition1.escrow.clone().unwrap(),
        &arena_interface::escrow::ExecuteMsg::Withdraw {
            cw20_msg: None,
            cw721_msg: None,
        },
        &[],
    );
    assert!(result.is_ok());

    // Assert correct balances user 1 - 10_000*.85, user 2 - 10_000*.85, dao - 20_000*.15
    let balance = context
        .app
        .wrap()
        .query_balance(user1.to_string(), "juno")
        .unwrap();
    assert_eq!(balance.amount, Uint128::from(8_500u128));
    let balance = context
        .app
        .wrap()
        .query_balance(user2.to_string(), "juno")
        .unwrap();
    assert_eq!(balance.amount, Uint128::from(8_500u128));
    let balance = context
        .app
        .wrap()
        .query_balance(context.core.dao_addr.to_string(), "juno")
        .unwrap();
    assert_eq!(balance.amount, Uint128::from(3_000u128));

    // Assert result is populated
    let result: Option<Distribution<String>> = context
        .app
        .wrap()
        .query_wasm_smart(
            context.wager.wager_module_addr.clone(),
            &QueryMsg::Result {
                competition_id: competition1_id,
            },
        )
        .unwrap();
    assert!(result.is_none());
}
