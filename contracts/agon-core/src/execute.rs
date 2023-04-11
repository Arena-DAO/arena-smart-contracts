use cosmwasm_std::{Decimal, DepsMut, Env, MessageInfo, Response, StdError, SubMsg, Uint128};
use dao_interface::ModuleInstantiateInfo;

use crate::{
    msg::PrePropose,
    state::{competition_modules, rulesets, Ruleset, RULESET_COUNT, TAX},
    ContractError,
};

pub const COMPETITION_MODULE_REPLY_ID: u64 = 1;
pub const DAO_REPLY_ID: u64 = 2;
pub const ESCROW_REPLY_ID: u64 = 3;
pub const COMPETITION_REPLY_ID: u64 = 5;

pub fn update_competition_modules(
    deps: DepsMut,
    env: &Env,
    info: MessageInfo,
    to_add: Vec<ModuleInstantiateInfo>,
    to_disable: Vec<Uint128>,
) -> Result<Response, ContractError> {
    if PrePropose::default().dao.load(deps.storage)? != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    for id in to_disable {
        competition_modules().update(deps.storage, id.u128(), |x| match x {
            Some(mut module) => {
                module.is_enabled = false;
                Ok(module)
            }
            None => Err(StdError::GenericErr {
                msg: format!("Could not find a competition module with the id {}", id),
            }),
        })?;
    }
    let competition_module_msgs: Vec<SubMsg> = to_add
        .into_iter()
        .map(|info| info.into_wasm_msg(env.contract.address.clone()))
        .map(|wasm| SubMsg::reply_on_success(wasm, COMPETITION_MODULE_REPLY_ID))
        .collect();

    Ok(Response::new()
        .add_attribute("action", "update_competition_modules")
        .add_submessages(competition_module_msgs))
}

pub fn update_tax(deps: DepsMut, env: &Env, tax: Decimal) -> Result<Response, ContractError> {
    if tax > Decimal::one() {
        return Err(ContractError::StdError(StdError::GenericErr {
            msg: "The dao tax cannot be greater than 100%.".to_string(),
        }));
    }
    TAX.save(deps.storage, &tax, env.block.height)?;

    Ok(Response::new()
        .add_attribute("action", "update_tax")
        .add_attribute("tax", tax.to_string()))
}

/*pub fn jail_wager(deps: DepsMut, env: Env, id: Uint128) -> Result<Response, ContractError> {
    let wager = WAGERS.update(deps.storage, id.u128(), |x| -> Result<_, ContractError> {
        if x.is_none() {
            return Err(ContractError::UnknownWagerId { id: id.u128() });
        }
        let mut wager = x.unwrap();

        if wager.expiration.is_expired(&env.block) {
            wager.status = WagerStatus::Jailed;
        }

        Ok(wager)
    })?;

    if wager.status != WagerStatus::Jailed {
        return Err(ContractError::InvalidWagerStatus {});
    }

    Ok(Response::new()
        .add_attribute("action", "jail_wager")
        .add_message(create_wager_proposals(
            deps.as_ref(),
            &env.contract.address,
            &PrePropose::default().dao.load(deps.storage)?,
            id,
        )?))
}

pub fn create_wager(
    deps: DepsMut,
    env: Env,
    wager_dao: WagerDAO,
    expiration: Expiration,
    escrow_code_id: u64,
    wager_amount: Vec<MemberBalance>,
    stake: Vec<MemberBalance>,
    rules: Vec<String>,
    ruleset: Option<Uint128>,
) -> Result<Response, ContractError> {
    let wager_count = WAGER_COUNT.update(deps.storage, |x| -> StdResult<_> {
        Ok(x.checked_add(Uint128::one())?)
    })?;
    let dao = PrePropose::default().dao.load(deps.storage)?;

    let mut wager = Wager {
        dao: env.contract.address.clone(),
        expiration,
        rules,
        ruleset,
        evidence: None,
        status: WagerStatus::Pending,
        escrow: env.contract.address.clone(),
    };

    let mut msgs = match wager_dao {
        WagerDAO::New {
            members,
            dao_code_id,
            group_code_id,
            proposal_code_id,
        } => vec![SubMsg::reply_always(
            WasmMsg::Instantiate {
                admin: Some(dao.to_string()),
                code_id: dao_code_id,
                msg: to_binary(&dao_core::msg::InstantiateMsg {
                    admin: Some(dao.to_string()),
                    name: format!("Agon Wager {} DAO", wager_count).to_string(),
                    description: "This is an Agon wager DAO.".to_string(),
                    image_url: None,
                    automatically_add_cw20s: true,
                    automatically_add_cw721s: true,
                    voting_module_instantiate_info: dao_interface::ModuleInstantiateInfo {
                        code_id: group_code_id,
                        msg: to_binary(&cw4_group::msg::InstantiateMsg {
                            members: members
                                .iter()
                                .map(|x| Member {
                                    addr: x.to_string(),
                                    weight: 1u64,
                                })
                                .collect(),
                            admin: None,
                        })?,
                        admin: Some(dao_interface::Admin::Address {
                            addr: dao.to_string(),
                        }),
                        label: "Voting Module".to_string(),
                    },
                    proposal_modules_instantiate_info: vec![dao_interface::ModuleInstantiateInfo {
                        code_id: proposal_code_id,
                        msg: to_binary(&dao_proposal_multiple::msg::InstantiateMsg {
                            voting_strategy:
                                dao_voting::multiple_choice::VotingStrategy::SingleChoice {
                                    quorum: dao_voting::threshold::PercentageThreshold::Percent(
                                        Decimal::percent(100),
                                    ),
                                },
                            min_voting_period: None,
                            max_voting_period: Duration::Time(604800u64), //1 week
                            only_members_execute: false,
                            allow_revoting: true,
                            pre_propose_info:
                                dao_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {},
                            close_proposal_on_execution_failure: true,
                        })?,
                        admin: Some(dao_interface::Admin::CoreModule {}),
                        label: "Proposal Multiple Module".to_string(),
                    }],
                    initial_items: None,
                    dao_uri: None,
                })?,
                funds: vec![],
                label: "DAO".to_string(),
            },
            DAO_REPLY_ID,
        )],
        WagerDAO::Existing { addr } => {
            wager.dao = deps.api.addr_validate(&addr)?;
            vec![SubMsg::new(create_wager_proposals(
                deps.as_ref(),
                &env.contract.address,
                &wager.dao,
                wager_count,
            )?)]
        }
    };

    msgs.push(SubMsg::reply_always(
        WasmMsg::Instantiate {
            admin: Some(dao.to_string()),
            code_id: escrow_code_id,
            msg: to_binary(&agon_escrow::msg::InstantiateMsg {
                due: wager_amount,
                stake,
                key: env.contract.address.to_string() + "_" + &wager_count.to_string(),
            })?,
            funds: vec![],
            label: "Escrow".to_string(),
        },
        ESCROW_REPLY_ID,
    ));

    WAGERS.save(deps.storage, wager_count.u128(), &wager)?;
    TEMP_WAGER.save(deps.storage, &wager_count.u128())?;

    Ok(Response::new()
        .add_attribute("wager_count", wager_count)
        .add_submessages(msgs))
}

pub fn create_wager_proposals(
    deps: Deps,
    contract_address: &Addr,
    dao: &Addr,
    wager_id: Uint128,
) -> Result<CosmosMsg, ContractError> {
    let proposal_module = PrePropose::default().proposal_module.load(deps.storage)?;

    let team_addr: Addr = deps
        .querier
        .query_wasm_smart(dao, &dao_core::msg::QueryMsg::VotingModule {})?;
    let teams: MemberListResponse = deps.querier.query_wasm_smart(
        &team_addr,
        &cw4_disbursement::msg::QueryMsg::ListMembers {
            start_after: None,
            limit: Some(u32::MAX),
        },
    )?;
    let mut team_number = 0;
    let options = teams
        .members
        .iter()
        .map(|x| {
            team_number += 1;
            Ok(dao_voting::multiple_choice::MultipleChoiceOption {
                title: format!("Team {}", team_number),
                description: "This team is the winner.".to_string(),
                msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_address.to_string(),
                    msg: to_binary(&ExecuteMsg::Extension {
                        msg: ExecuteExt::HandleWager {
                            id: wager_id,
                            distribution: Some(vec![MemberShare {
                                addr: x.addr.clone(),
                                shares: Uint128::one(),
                            }]),
                        },
                    })?,
                    funds: vec![],
                })],
            })
        })
        .collect::<StdResult<Vec<dao_voting::multiple_choice::MultipleChoiceOption>>>()?;

    //create the proposals
    return Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: proposal_module.to_string(),
        msg: to_binary(&dao_proposal_multiple::msg::ExecuteMsg::Propose {
            title: "Agon Result".to_string(),
            description: "Decide the competition's winner".to_string(),
            choices: dao_voting::multiple_choice::MultipleChoiceOptions { options },
            proposer: None,
        })?,
        funds: vec![],
    }));
}

pub fn handle_wager(
    deps: DepsMut,
    info: MessageInfo,
    id: Uint128,
    distribution: Option<Vec<MemberShare>>,
) -> Result<Response, ContractError> {
    let wager = WAGERS.load(deps.storage, id.u128())?;
    let dao = PrePropose::default().dao.load(deps.storage)?;

    match wager.status {
        WagerStatus::Active => {
            if wager.dao != info.sender && dao != info.sender {
                return Err(ContractError::Unauthorized {});
            }
            Ok(())
        }
        WagerStatus::Jailed => {
            if dao != info.sender {
                return Err(ContractError::Unauthorized {});
            }
            Ok(())
        }
        _ => Err(ContractError::InvalidWagerStatus {}),
    }?;

    //insert tax shares
    let distribution = match distribution {
        Some(mut member_shares) => {
            let mut sum = Uint128::zero();
            for member_share in &member_shares {
                sum = sum.checked_add(Uint128::from(member_share.shares))?;
            }
            let tax = TAX.load(deps.storage)?;
            let dao_shares = tax
                .checked_mul(Decimal::from_atomics(sum, 0u32)?)?
                .checked_div(Decimal::one().checked_sub(tax)?)?;
            let dao_shares = dao_shares
                .checked_div(Decimal::from_atomics(
                    Uint128::new(10u128).checked_pow(dao_shares.decimal_places())?,
                    0u32,
                )?)?
                .atomics();

            member_shares.push(MemberShare {
                addr: dao.to_string(),
                shares: dao_shares,
            });
            Some(member_shares)
        }
        None => None,
    };

    let msg = SubMsg::reply_always(
        WasmMsg::Execute {
            contract_addr: wager.escrow.to_string(),
            msg: to_binary(&cw_competition::CompetitionResultMsg { distribution })?,
            funds: vec![],
        },
        COMPETITION_REPLY_ID,
    );
    Ok(Response::new()
        .add_attribute("action", "handle_wager")
        .add_submessage(msg))
}*/

pub fn update_rulesets(
    deps: DepsMut,
    to_add: Vec<Ruleset>,
    to_disable: Vec<Uint128>,
) -> Result<Response, ContractError> {
    for id in to_disable {
        rulesets().update(deps.storage, id.u128(), |x| match x {
            Some(mut module) => {
                module.is_enabled = false;
                Ok(module)
            }
            None => Err(StdError::GenericErr {
                msg: format!("Could not find a competition module with the id {}", id),
            }),
        })?;
    }

    let mut id = RULESET_COUNT.load(deps.storage)?;
    for ruleset in to_add {
        rulesets().save(deps.storage, id.u128(), &ruleset)?;
        id = id.checked_add(Uint128::one())?;
    }
    RULESET_COUNT.save(deps.storage, &id)?;

    Ok(Response::new()
        .add_attribute("action", "update_rulesets")
        .add_attribute("ruleset_count", Uint128::from(id)))
}
