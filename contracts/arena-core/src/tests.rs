use cosmwasm_std::{to_binary, Addr, Coin, Decimal, Empty, StdError, Uint128, WasmMsg};
use cw_balance::{Balance, MemberBalance};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use cw_utils::Expiration;
use dao_interface::{
    query::GetItemResponse,
    state::{Admin, ModuleInstantiateInfo, ProposalModule},
};

use crate::msg::{InstantiateExt, InstantiateMsg, QueryMsg};

const CREATOR: &str = "ismellike";
const WAGER_KEY: &str = "wager";
struct Context {
    app: App,
    dao_core_id: u64,
    dao_addr: Addr,
    // You can add more fields here as needed.
}

fn sudo_proposal_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        dao_proposal_sudo::contract::execute,
        dao_proposal_sudo::contract::instantiate,
        dao_proposal_sudo::contract::query,
    );
    Box::new(contract)
}

fn setup_app() -> Context {
    let mut app = App::default();
    let dao_core_id = app.store_code(dao_testing::contracts::dao_dao_contract());
    let sudo_id = app.store_code(sudo_proposal_contract());

    let sudo_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: CREATOR.to_owned(),
    };

    let gov_instantiate = dao_interface::msg::InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "Arena DAO".to_string(),
        description: "Decentralized Competition".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: sudo_id,
            msg: to_binary(&sudo_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            label: "voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: sudo_id,
            msg: to_binary(&sudo_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            label: "voting module".to_string(),
        }],
        initial_items: None,
    };

    let dao_addr = app
        .instantiate_contract(
            dao_core_id,
            Addr::unchecked(CREATOR),
            &gov_instantiate,
            &[],
            "cw-governance",
            None,
        )
        .unwrap();

    Context {
        app,
        dao_core_id,
        dao_addr,
    }
}

fn execute_attach_arena_core(context: &mut Context) {
    let dpm_id = context
        .app
        .store_code(arena_testing::contracts::dao_proposal_multiple_contract());
    let arena_id = context
        .app
        .store_code(arena_testing::contracts::arena_dao_core_contract());
    let wager_module_id = context
        .app
        .store_code(arena_testing::contracts::arena_wager_module_contract());

    let proposal_module = &context
        .app
        .wrap()
        .query_wasm_smart::<Vec<ProposalModule>>(
            context.dao_addr.clone(),
            &dao_interface::msg::QueryMsg::ProposalModules {
                start_after: None,
                limit: Some(1u32),
            },
        )
        .unwrap()[0];

    context
    .app.execute_contract(
        Addr::unchecked(CREATOR),
        proposal_module.address.clone(),
        &dao_proposal_sudo::msg::ExecuteMsg::Execute {
            msgs: vec![WasmMsg::Execute {
                contract_addr: context.dao_addr.to_string(),
                funds: vec![],
                msg: to_binary(&dao_interface::msg::ExecuteMsg::UpdateProposalModules {
                    to_add: vec![ModuleInstantiateInfo {
                        code_id: dpm_id,
                        msg: to_binary(&dao_proposal_multiple::msg::InstantiateMsg {
                            voting_strategy:
                                dao_voting::multiple_choice::VotingStrategy::SingleChoice {
                                    quorum: dao_voting::threshold::PercentageThreshold::Majority {},
                                },
                            min_voting_period: None,
                            max_voting_period: cw_utils_v16::Duration::Time(1209600u64),
                            only_members_execute: false,
                            allow_revoting: false,
                            pre_propose_info:
                                dao_voting::pre_propose::PreProposeInfo::ModuleMayPropose {
                                    info: ModuleInstantiateInfo {
                                        code_id: arena_id,
                                        msg: to_binary(&InstantiateMsg {
                                            deposit_info: None,
                                            open_proposal_submission: false,
                                            extension: InstantiateExt {
                                                competition_modules_instantiate_info: vec![
                                                    ModuleInstantiateInfo {
                                                        code_id: wager_module_id,
                                                        msg: to_binary(&arena_wager_module::msg::InstantiateMsg {
                                                            key: WAGER_KEY.to_string(),
                                                            description: "The Arena Protocol Wager Module".to_string(), 
                                                            extension: Empty {}}).unwrap(),
                                                        admin: Some(Admin::CoreModule {}),
                                                        label: "Arena Wager Module".to_string(),
                                                    },
                                                ],
                                                rulesets: vec![],
                                                tax: Decimal::new(Uint128::from(
                                                    150000000000000000u128,
                                                )),
                                            },
                                        })
                                        .unwrap(),
                                        admin: Some(Admin::CoreModule {
                                        }),
                                        label: "Arena Core".to_string(),
                                    },
                                },
                            close_proposal_on_execution_failure: true,
                        })
                        .unwrap(),
                        admin: Some(Admin::Address { addr: context.dao_addr.to_string() }),
                        label: "Proposal Multiple".to_string(),
                    }],
                    to_disable: vec![],
                })
                .unwrap(),
            }
            .into()],
        },
        &vec![],
    ).unwrap();
}

#[test]
fn attach_arena_core() {
    let mut context = setup_app();

    execute_attach_arena_core(&mut context);

    let item_response: GetItemResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.dao_addr,
            &dao_interface::msg::QueryMsg::GetItem {
                key: crate::contract::ITEM_KEY.to_owned(),
            },
        )
        .unwrap();

    assert!(item_response.item.is_some());
}

#[test]
fn create_and_process_wager() {
    let mut context = setup_app();
    execute_attach_arena_core(&mut context);
    let escrow_code_id = context
        .app
        .store_code(arena_testing::contracts::arena_dao_escrow_contract());
    let cw4_id = context
        .app
        .store_code(dao_testing::contracts::cw4_group_contract());
    let cw4_voting_id = context
        .app
        .store_code(dao_testing::contracts::dao_voting_cw4_contract());
    let dpm_id = context
        .app
        .store_code(arena_testing::contracts::dao_proposal_multiple_contract());

    let item_response: GetItemResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.dao_addr,
            &dao_interface::msg::QueryMsg::GetItem {
                key: crate::contract::ITEM_KEY.to_owned(),
            },
        )
        .unwrap();
    let arena_core_addr = item_response.item.unwrap();

    let res: Result<Option<Addr>, StdError> = context.app.wrap().query_wasm_smart(
        arena_core_addr,
        &QueryMsg::QueryExtension {
            msg: crate::msg::QueryExt::CompetitionModule {
                key: WAGER_KEY.to_string(),
            },
        },
    );

    assert!(res.is_ok());
    assert!(res.as_ref().unwrap().is_some());

    let wager_module_addr = res.unwrap().unwrap();

    let addr1 = Addr::unchecked("addr1".to_string());
    let addr2 = Addr::unchecked("addr2".to_string());

    let wager_instantiate_msg = arena_wager_module::msg::ExecuteMsg::CreateCompetition {
        competition_dao: ModuleInstantiateInfo {
            code_id: context.dao_core_id,
            msg: to_binary(&dao_interface::msg::InstantiateMsg {
                admin: None,
                name: "Competition DAO".to_string(),
                description: "Determine the winner of the competition".to_string(),
                image_url: None,
                automatically_add_cw20s: true,
                automatically_add_cw721s: true,
                voting_module_instantiate_info: ModuleInstantiateInfo {
                    code_id: cw4_voting_id,
                    msg: to_binary(&dao_voting_cw4::msg::InstantiateMsg {
                        cw4_group_code_id: cw4_id,
                        initial_members: vec![
                            cw4::Member {
                                addr: addr1.to_string(),
                                weight: 1u64,
                            },
                            cw4::Member {
                                addr: addr2.to_string(),
                                weight: 1u64,
                            },
                        ],
                    })
                    .unwrap(),
                    admin: None,
                    label: "Cw4 Voting".to_string(),
                },
                proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
                    code_id: dpm_id,
                    msg: to_binary(&dao_proposal_multiple::msg::InstantiateMsg {
                        voting_strategy:
                            dao_voting::multiple_choice::VotingStrategy::SingleChoice {
                                quorum: dao_voting::threshold::PercentageThreshold::Percent(
                                    Decimal::one(),
                                ),
                            },
                        min_voting_period: None,
                        max_voting_period: cw_utils_v16::Duration::Time(100000u64),
                        only_members_execute: false,
                        allow_revoting: false,
                        pre_propose_info:
                            dao_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {},
                        close_proposal_on_execution_failure: false,
                    })
                    .unwrap(),
                    admin: Some(Admin::CoreModule {}),
                    label: "Proposal Multiple".to_string(),
                }],
                initial_items: None,
                dao_uri: None,
            })
            .unwrap(),
            admin: None,
            label: "Competition DAO".to_string(),
        },
        escrow: ModuleInstantiateInfo {
            code_id: escrow_code_id,
            msg: to_binary(&arena_escrow::msg::InstantiateMsg {
                dues: vec![
                    MemberBalance {
                        addr: addr1.to_string(),
                        balance: Balance {
                            native: vec![Coin {
                                denom: "juno".to_string(),
                                amount: Uint128::from(14u128),
                            }],
                            cw20: vec![],
                            cw721: vec![],
                        },
                    },
                    MemberBalance {
                        addr: addr2.to_string(),
                        balance: Balance {
                            native: vec![Coin {
                                denom: "juno".to_string(),
                                amount: Uint128::from(20u128),
                            }],
                            cw20: vec![],
                            cw721: vec![],
                        },
                    },
                ],
                lock_when_funded: true,
            })
            .unwrap(),
            admin: None,
            label: "Arena Escrow".to_string(),
        },
        name: "Test wager".to_string(),
        description: "Test description".to_string(),
        expiration: Expiration::Never {},
        rules: vec![],
        ruleset: None,
        extension: Empty {},
    };

    let res = context.app.execute_contract(
        addr1.clone(),
        wager_module_addr.clone(),
        &wager_instantiate_msg,
        &[],
    );

    assert!(res.is_ok());

    let wager: arena_wager_module::msg::Wager = context
        .app
        .wrap()
        .query_wasm_smart(
            wager_module_addr.clone(),
            &arena_wager_module::msg::QueryMsg::Competition { id: Uint128::one() },
        )
        .unwrap();

    assert!(wager.dao != wager.escrow);
}
