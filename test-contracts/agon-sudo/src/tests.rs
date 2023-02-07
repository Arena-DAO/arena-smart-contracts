use cosmwasm_std::{testing::mock_info, to_binary, Addr, Coin, Decimal, Empty, Uint128};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw4_disbursement::{
    contract::{execute, instantiate, query},
    msg::{ExecuteMsg, InstantiateMsg},
};
use cw721::Cw721ExecuteMsg;
use cw_disbursement::{CwDisbursementExecuteMsg, MemberShare};
use cw_multi_test::{next_block, App, Contract, ContractWrapper, Executor};
use dao_interface::ModuleInstantiateInfo;
use dao_voting_cw20_staked::msg::StakingInfo;

pub const WAGER1: &str = "wager-1";
pub const WAGER1_BEGINNING_BALANCE: u128 = 1000u128;
pub const DENOM: &str = "AGON";
pub const ADDR1: &str = "member-1";
pub const ADDR2: &str = "member-2";
pub const ADDR3: &str = "member-3";
pub const ADDR4: &str = "member-4";
pub const ADDR5: &str = "member-5";
pub const NFT_ID: &str = "token-id-1";

struct Context {
    pub token: Addr,
    pub nft: Addr,
    pub team1: Addr,
    pub team2: Addr,
    pub team_dao1: Addr,
    pub team_dao2: Addr,
    pub agon_dao: Addr,
}

pub fn contract_cw20() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

pub fn contract_cw721() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw721_base::entry::execute,
        cw721_base::entry::instantiate,
        cw721_base::entry::query,
    );
    Box::new(contract)
}

pub fn contract_cw20_stake() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_stake::contract::execute,
        cw20_stake::contract::instantiate,
        cw20_stake::contract::query,
    );
    Box::new(contract)
}

fn instantiate_cw721(app: &mut App, code_id: u64, msg: cw721_base::msg::InstantiateMsg) -> Addr {
    app.instantiate_contract(code_id, Addr::unchecked(ADDR1), &msg, &[], "cw721", None)
        .unwrap()
}

fn instantiate_cw20(app: &mut App, code_id: u64, msg: cw20_base::msg::InstantiateMsg) -> Addr {
    app.instantiate_contract(code_id, Addr::unchecked(ADDR1), &msg, &[], "cw20", None)
        .unwrap()
}

fn contract_dao_core() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        dao_core::contract::execute,
        dao_core::contract::instantiate,
        dao_core::contract::query,
    )
    .with_reply(dao_core::contract::reply)
    .with_migrate(dao_core::contract::migrate);
    Box::new(contract)
}

fn contract_agon_core() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        agon_core::contract::execute,
        agon_core::contract::instantiate,
        agon_core::contract::query,
    )
    .with_reply(agon_core::contract::reply);
    Box::new(contract)
}

fn contract_proposal_multiple_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        dao_proposal_multiple::contract::execute,
        dao_proposal_multiple::contract::instantiate,
        dao_proposal_multiple::contract::query,
    )
    .with_reply(dao_proposal_multiple::contract::reply);
    Box::new(contract)
}

fn instantiate_dao_core(app: &mut App, code_id: u64, msg: dao_core::msg::InstantiateMsg) -> Addr {
    app.instantiate_contract(code_id, Addr::unchecked(ADDR1), &msg, &[], "cwd-core", None)
        .unwrap()
}

pub fn contract_cw4_team() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(execute, instantiate, query))
}

fn instantiate_cw4_team(app: &mut App, code_id: u64, sender: Addr, msg: InstantiateMsg) -> Addr {
    app.instantiate_contract(code_id, sender.clone(), &msg, &[], "cw4-team", None)
        .unwrap()
}

fn cw20_balances_voting() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        dao_voting_cw20_staked::contract::execute,
        dao_voting_cw20_staked::contract::instantiate,
        dao_voting_cw20_staked::contract::query,
    )
    .with_reply(dao_voting_cw20_staked::contract::reply);
    Box::new(contract)
}

fn sudo_proposal_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        dao_proposal_sudo::contract::execute,
        dao_proposal_sudo::contract::instantiate,
        dao_proposal_sudo::contract::query,
    );
    Box::new(contract)
}

fn get_balances(app: &App, addr: impl Into<String>) -> Vec<Coin> {
    app.wrap().query_all_balances(addr).unwrap()
}

fn mock_app() -> App {
    App::new(|router, _, storage| {
        // initialization moved to App construction
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked(WAGER1),
                vec![Coin {
                    denom: DENOM.to_string(),
                    amount: Uint128::from(WAGER1_BEGINNING_BALANCE),
                }],
            )
            .unwrap();
    })
}

fn setup_test_case(app: &mut App) -> Context {
    let cw20_id = app.store_code(contract_cw20());
    let govmod_id = app.store_code(sudo_proposal_contract());
    let dao_core_id = app.store_code(contract_dao_core());
    let cw4_team_id = app.store_code(contract_cw4_team());
    let voting_id = app.store_code(cw20_balances_voting());
    let cw721_id = app.store_code(contract_cw721());
    let cw20_stake_id = app.store_code(contract_cw20_stake());
    let agon_core_id = app.store_code(contract_agon_core());
    let proposal_multiple_id = app.store_code(contract_proposal_multiple_contract());

    let token = instantiate_cw20(
        app,
        cw20_id,
        cw20_base::msg::InstantiateMsg {
            name: String::from("Agon"),
            symbol: String::from("AGON"),
            decimals: 6,
            initial_balances: vec![{
                Cw20Coin {
                    address: String::from(WAGER1),
                    amount: Uint128::from(WAGER1_BEGINNING_BALANCE),
                }
            }],
            mint: None,
            marketing: None,
        },
    );

    let nft = instantiate_cw721(
        app,
        cw721_id,
        cw721_base::InstantiateMsg {
            name: "Agon NFTs".to_string(),
            symbol: "AGON".to_string(),
            minter: ADDR1.to_string(),
        },
    );
    app.execute_contract(
        Addr::unchecked(ADDR1),
        nft.clone(),
        &cw721_base::ExecuteMsg::<Option<String>, Option<String>>::Mint(cw721_base::MintMsg::<
            Option<String>,
        > {
            token_id: NFT_ID.to_string(),
            owner: WAGER1.to_string(),
            token_uri: None,
            extension: None,
        }),
        &[],
    )
    .unwrap();

    let voting_instantiate = dao_voting_cw20_staked::msg::InstantiateMsg {
        token_info: dao_voting_cw20_staked::msg::TokenInfo::Existing {
            address: token.to_string(),
            staking_contract: StakingInfo::New {
                staking_code_id: cw20_stake_id,
                unstaking_duration: None,
            },
        },
        active_threshold: None,
    };
    let govmod_instantiate = dao_proposal_sudo::msg::InstantiateMsg {
        root: ADDR1.to_string(),
    };

    let agon_dao = instantiate_dao_core(
        app,
        dao_core_id,
        dao_core::msg::InstantiateMsg {
            admin: None,
            name: "Agon Protocol".to_string(),
            description: "Welcome to Decentralized Competition!".to_string(),
            image_url: None,
            automatically_add_cw20s: true,
            automatically_add_cw721s: true,
            voting_module_instantiate_info: ModuleInstantiateInfo {
                code_id: voting_id,
                msg: to_binary(&voting_instantiate).unwrap(),
                admin: None,
                label: "voting module".to_string(),
            },
            proposal_modules_instantiate_info: vec![
                ModuleInstantiateInfo {
                    code_id: govmod_id,
                    msg: to_binary(&govmod_instantiate).unwrap(),
                    admin: None,
                    label: "proposal module".to_string(),
                },
                ModuleInstantiateInfo {
                    code_id: proposal_multiple_id,
                    msg: to_binary(&dao_proposal_multiple::msg::InstantiateMsg {
                        voting_strategy:
                            dao_voting::multiple_choice::VotingStrategy::SingleChoice {
                                quorum: dao_voting::threshold::PercentageThreshold::Majority {},
                            },
                        min_voting_period: None,
                        max_voting_period: cw_utils::Duration::Time(10000u64),
                        only_members_execute: false,
                        allow_revoting: false,
                        pre_propose_info:
                            dao_voting::pre_propose::PreProposeInfo::ModuleMayPropose {
                                info: ModuleInstantiateInfo {
                                    code_id: agon_core_id,
                                    msg: to_binary(&agon_core::msg::InstantiateMsg {
                                        deposit_info: None,
                                        open_proposal_submission: false,
                                        extension: agon_core::msg::InstantiateExt {
                                            competition_modules_instantiate_info: vec![],
                                            rulesets: vec![],
                                            tax: Decimal::percent(40),
                                        },
                                    })
                                    .unwrap(),
                                    admin: None,
                                    label: "Agon Core".to_string(),
                                },
                            },
                        close_proposal_on_execution_failure: true,
                    })
                    .unwrap(),
                    admin: None,
                    label: "agon module".to_string(),
                },
            ],
            initial_items: None,
            dao_uri: None,
        },
    );

    let team_dao2 = instantiate_dao_core(
        app,
        dao_core_id,
        dao_core::msg::InstantiateMsg {
            admin: None,
            name: "Team DAO 2".to_string(),
            description: "A testing dao.".to_string(),
            image_url: None,
            automatically_add_cw20s: true,
            automatically_add_cw721s: true,
            voting_module_instantiate_info: ModuleInstantiateInfo {
                code_id: voting_id,
                msg: to_binary(&voting_instantiate).unwrap(),
                admin: None,
                label: "voting module".to_string(),
            },
            proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
                code_id: govmod_id,
                msg: to_binary(&govmod_instantiate).unwrap(),
                admin: None,
                label: "governance module".to_string(),
            }],
            initial_items: None,
            dao_uri: None,
        },
    );

    let team2 = instantiate_cw4_team(
        app,
        cw4_team_id,
        team_dao2.clone(),
        InstantiateMsg {
            members: vec![
                cw4::Member {
                    addr: ADDR4.to_string(),
                    weight: 1,
                },
                cw4::Member {
                    addr: ADDR5.to_string(),
                    weight: 1,
                },
            ],
        },
    );
    app.update_block(next_block);

    let team_dao1 = instantiate_dao_core(
        app,
        dao_core_id,
        dao_core::msg::InstantiateMsg {
            admin: None,
            name: "Team DAO 1".to_string(),
            description: "A testing dao.".to_string(),
            image_url: None,
            automatically_add_cw20s: true,
            automatically_add_cw721s: true,
            voting_module_instantiate_info: ModuleInstantiateInfo {
                code_id: voting_id,
                msg: to_binary(&voting_instantiate).unwrap(),
                admin: None,
                label: "voting module".to_string(),
            },
            proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
                code_id: govmod_id,
                msg: to_binary(&govmod_instantiate).unwrap(),
                admin: None,
                label: "governance module".to_string(),
            }],
            initial_items: None,
            dao_uri: None,
        },
    );
    let team1 = instantiate_cw4_team(
        app,
        cw4_team_id,
        team_dao1.clone(),
        InstantiateMsg {
            members: vec![
                cw4::Member {
                    addr: ADDR1.to_string(),
                    weight: 1,
                },
                cw4::Member {
                    addr: ADDR2.to_string(),
                    weight: 1,
                },
                cw4::Member {
                    addr: ADDR3.to_string(),
                    weight: 1,
                },
                cw4::Member {
                    addr: team2.to_string(),
                    weight: 0,
                },
            ],
        },
    );
    app.update_block(next_block);

    Context {
        token,
        nft,
        team1,
        team2,
        team_dao1,
        team_dao2,
        agon_dao,
    }
}

fn get_cw20_balance<T: Into<String>, U: Into<String>>(
    app: &App,
    contract_addr: T,
    address: U,
) -> Uint128 {
    let msg = cw20::Cw20QueryMsg::Balance {
        address: address.into(),
    };
    let result: cw20::BalanceResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.balance
}

fn get_owner_of_nft<T: Into<String>, U: Into<String>>(
    app: &App,
    contract_addr: T,
    token_id: U,
) -> String {
    let msg = cw721::Cw721QueryMsg::OwnerOf {
        token_id: token_id.into(),
        include_expired: Some(false),
    };
    let result: cw721::OwnerOfResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.owner
}

#[test]
fn initialize_agon() {
    let mut app = mock_app();
    let context = setup_test_case(&mut app);

    let result: dao_core::query::GetItemResponse = app
        .wrap()
        .query_wasm_smart(
            context.agon_dao,
            &dao_core::msg::QueryMsg::GetItem {
                key: "Agon".to_string(),
            },
        )
        .unwrap();
    assert_eq!(result.item.is_some(), true);
}

#[test]
fn disbursement_handle_cw20() {
    let mut app = mock_app();
    let context = setup_test_case(&mut app);
    let info = mock_info(&WAGER1, &[]);

    //set disbursement data
    let disbursement_data = vec![
        MemberShare {
            addr: ADDR4.to_string(),
            shares: Uint128::one(),
        },
        MemberShare {
            addr: ADDR5.to_string(),
            shares: Uint128::one(),
        },
    ];
    let msg = ExecuteMsg::CwDisbursementExecute(CwDisbursementExecuteMsg::SetDisbursementData {
        disbursement_data: disbursement_data.clone(),
        key: WAGER1.to_string(),
    });
    app.execute_contract(info.sender.clone(), context.team2.clone(), &msg, &[])
        .unwrap();

    //set disbursement data
    let disbursement_data = vec![
        MemberShare {
            addr: ADDR1.to_string(),
            shares: Uint128::one(),
        },
        MemberShare {
            addr: ADDR2.to_string(),
            shares: Uint128::one(),
        },
        MemberShare {
            addr: context.team2.to_string(),
            shares: Uint128::one(),
        },
    ];
    let msg = ExecuteMsg::CwDisbursementExecute(CwDisbursementExecuteMsg::SetDisbursementData {
        disbursement_data: disbursement_data.clone(),
        key: WAGER1.to_string(),
    });
    app.execute_contract(info.sender, context.team1.clone(), &msg, &[])
        .unwrap();

    //execute
    let msg = Cw20ExecuteMsg::Send {
        contract: context.team1.to_string(),
        amount: Uint128::new(WAGER1_BEGINNING_BALANCE),
        msg: to_binary(&WAGER1).unwrap(),
    };
    let info = mock_info(WAGER1, &[]);
    app.execute_contract(info.sender, context.token.clone(), &msg, &[])
        .unwrap();

    //this was 1000 split into 3 sets of 333 with remainder 1 sent to the last team
    //one set of 334 was further split into 2 sets of 167
    assert_eq!(
        Uint128::from(333u128),
        get_cw20_balance(&app, context.token.clone(), ADDR1)
    );
    assert_eq!(
        Uint128::from(333u128),
        get_cw20_balance(&app, context.token.clone(), ADDR2)
    );
    assert_eq!(
        Uint128::from(167u128),
        get_cw20_balance(&app, context.token.clone(), ADDR4)
    );
    assert_eq!(
        Uint128::from(167u128),
        get_cw20_balance(&app, context.token.clone(), ADDR5)
    );
}

#[test]
fn disbursement_not_configured() {
    let mut app = mock_app();
    let context = setup_test_case(&mut app);
    let info = mock_info(&WAGER1, &[]);

    //set disbursement data
    let disbursement_data = vec![
        MemberShare {
            addr: ADDR4.to_string(),
            shares: Uint128::one(),
        },
        MemberShare {
            addr: ADDR5.to_string(),
            shares: Uint128::one(),
        },
    ];
    let msg = ExecuteMsg::CwDisbursementExecute(CwDisbursementExecuteMsg::SetDisbursementData {
        disbursement_data: disbursement_data.clone(),
        key: WAGER1.to_string(),
    });
    app.execute_contract(info.sender.clone(), context.team2.clone(), &msg, &[])
        .unwrap();

    //execute
    let msg = Cw20ExecuteMsg::Send {
        contract: context.team1.to_string(),
        amount: Uint128::new(WAGER1_BEGINNING_BALANCE),
        msg: to_binary(&WAGER1).unwrap(),
    };
    let info = mock_info(WAGER1, &[]);
    app.execute_contract(info.sender.clone(), context.token.clone(), &msg, &[])
        .unwrap();

    //the transfer should fail, so the dao will hold the funds
    assert_eq!(
        Uint128::from(WAGER1_BEGINNING_BALANCE),
        get_cw20_balance(&app, context.token.clone(), context.team_dao1)
    );
}

#[test]
fn disbursement_handle_nft() {
    let mut app = mock_app();
    let context = setup_test_case(&mut app);
    let info = mock_info(&WAGER1, &[]);

    //set disbursement data
    let disbursement_data = vec![
        MemberShare {
            addr: ADDR4.to_string(),
            shares: Uint128::one(),
        },
        MemberShare {
            addr: ADDR5.to_string(),
            shares: Uint128::one(),
        },
    ];
    let msg = ExecuteMsg::CwDisbursementExecute(CwDisbursementExecuteMsg::SetDisbursementData {
        disbursement_data: disbursement_data.clone(),
        key: WAGER1.to_string(),
    });
    app.execute_contract(info.sender.clone(), context.team2.clone(), &msg, &[])
        .unwrap();

    //execute
    let msg = Cw721ExecuteMsg::SendNft {
        contract: context.team2.to_string(),
        msg: to_binary(&WAGER1).unwrap(),
        token_id: NFT_ID.to_string(),
    };
    let info = mock_info(WAGER1, &[]);
    app.execute_contract(info.sender.clone(), context.nft.clone(), &msg, &[])
        .unwrap();

    assert_eq!(
        get_owner_of_nft(&app, context.nft.clone(), NFT_ID),
        context.team_dao2.to_string()
    );
}

#[test]
fn disbursement_handle_native() {
    let mut app = mock_app();
    let context = setup_test_case(&mut app);
    let info = mock_info(
        &WAGER1,
        &[Coin {
            denom: DENOM.to_string(),
            amount: Uint128::from(WAGER1_BEGINNING_BALANCE),
        }],
    );

    //set disbursement data
    let disbursement_data = vec![
        MemberShare {
            addr: ADDR4.to_string(),
            shares: Uint128::one(),
        },
        MemberShare {
            addr: ADDR5.to_string(),
            shares: Uint128::one(),
        },
    ];
    let msg = ExecuteMsg::CwDisbursementExecute(CwDisbursementExecuteMsg::SetDisbursementData {
        disbursement_data: disbursement_data.clone(),
        key: WAGER1.to_string(),
    });
    app.execute_contract(info.sender.clone(), context.team2.clone(), &msg, &[])
        .unwrap();

    //set disbursement data
    let disbursement_data = vec![
        MemberShare {
            addr: ADDR1.to_string(),
            shares: Uint128::one(),
        },
        MemberShare {
            addr: ADDR2.to_string(),
            shares: Uint128::one(),
        },
        MemberShare {
            addr: context.team2.to_string(),
            shares: Uint128::one(),
        },
    ];
    let msg = ExecuteMsg::CwDisbursementExecute(CwDisbursementExecuteMsg::SetDisbursementData {
        disbursement_data: disbursement_data.clone(),
        key: WAGER1.to_string(),
    });
    app.execute_contract(info.sender.clone(), context.team1.clone(), &msg, &[])
        .unwrap();

    //execute
    let msg = ExecuteMsg::CwDisbursementExecute(CwDisbursementExecuteMsg::ReceiveNative {
        key: Some(info.sender.to_string()),
    });
    app.execute_contract(
        info.sender.clone(),
        context.team1.clone(),
        &msg,
        &info.funds,
    )
    .unwrap();

    assert_eq!(
        get_balances(&app, ADDR1),
        vec![Coin {
            denom: DENOM.to_string(),
            amount: Uint128::from(333u128)
        }]
    );
    assert_eq!(
        get_balances(&app, ADDR2),
        vec![Coin {
            denom: DENOM.to_string(),
            amount: Uint128::from(333u128)
        }]
    );
    assert_eq!(
        get_balances(&app, ADDR4),
        vec![Coin {
            denom: DENOM.to_string(),
            amount: Uint128::from(167u128)
        }]
    );
    assert_eq!(
        get_balances(&app, ADDR5),
        vec![Coin {
            denom: DENOM.to_string(),
            amount: Uint128::from(167u128)
        }]
    );
}
