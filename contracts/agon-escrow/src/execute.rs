use cosmwasm_std::{Addr, Attribute, Coin, DepsMut, MessageInfo, Response, StdResult, Storage};
use cw20::Cw20ReceiveMsg;
use cw721::Cw721ReceiveMsg;
use cw_competition::{CompetitionState, CwCompetitionResultMsg, CwCompetitionStateChangedMsg};
use cw_disbursement::{disburse, CwDisbursementContract, MemberBalance};
use cw_tokens::{
    BalanceError, BatchCoinExtensions, GenericBalanceExtensions, GenericTokenBalance,
    TokenExtensions,
};
use std::collections::HashSet;

use crate::{
    models::EscrowState,
    state::{ADMIN, BALANCE, DUE, KEY, STATE, TOTAL_BALANCE},
    ContractError,
};

fn inner_refund(deps: DepsMut, addrs: Vec<Addr>) -> Result<Response, ContractError> {
    //init
    let key = KEY.load(deps.storage)?;
    let mut contracts = HashSet::new();
    let mut total_balance = TOTAL_BALANCE.load(deps.storage)?;
    let mut msgs = vec![];
    let mut attrs = vec![];

    for addr in addrs {
        //get specific balance
        let balance = BALANCE.load(deps.storage, addr.clone())?;

        //get disbursement contracts
        if CwDisbursementContract(addr.clone())
            .is_disbursement_contract(&deps.querier, &Some(key.to_string()))
        {
            contracts.insert(addr.clone());
        }

        //create transfer messages
        msgs.append(
            &mut MemberBalance {
                member: addr.to_string(),
                balances: balance.clone(),
            }
            .to_msgs(Some(key.to_string()), &contracts)?,
        );

        //add attributes
        attrs.push(Attribute {
            key: "addr".to_string(),
            value: addr.to_string(),
        });

        //update values
        total_balance = total_balance.sub_balances_checked(&balance)?;
    }

    //save total
    TOTAL_BALANCE.save(deps.storage, &total_balance)?;

    //response
    Ok(Response::new()
        .add_attribute("action", "refund")
        .add_attribute("key", key.to_string())
        .add_attributes(attrs)
        .add_messages(msgs))
}

pub fn refund(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    if STATE.may_load(deps.storage)? != Some(EscrowState::Unlocked {}) {
        return Err(ContractError::InvalidState {});
    }

    inner_refund(deps, vec![info.sender])
}

pub fn cw721_receive(
    deps: DepsMut,
    info: MessageInfo,
    cw721_receive_msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    let sender = deps.api.addr_validate(&cw721_receive_msg.sender)?;

    Ok(receive_generic(
        deps.storage,
        &sender,
        &info.funds,
        &mut vec![cw721_receive_msg.to_generic(&info.sender)],
    )?)
}

pub fn cw20_receive(
    deps: DepsMut,
    info: MessageInfo,
    cw20_receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let sender = deps.api.addr_validate(&cw20_receive_msg.sender)?;
    Ok(receive_generic(
        deps.storage,
        &sender,
        &info.funds,
        &mut vec![cw20_receive_msg.to_generic(&info.sender)],
    )?)
}

pub fn receive_native(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    Ok(receive_generic(
        deps.storage,
        &info.sender,
        &info.funds,
        &mut vec![],
    )?)
}

fn receive_generic(
    storage: &mut dyn Storage,
    sender: &Addr,
    funds: &Vec<Coin>,
    tokens: &mut Vec<GenericTokenBalance>,
) -> Result<Response, ContractError> {
    let mut to_add = funds.to_generic_batch();
    to_add.append(tokens);

    BALANCE.update(storage, sender.clone(), |x| -> Result<_, BalanceError> {
        if x.is_none() {
            return Ok(to_add.clone());
        } else {
            return Ok(x.unwrap().add_balances_checked(&to_add)?);
        }
    })?;

    DUE.update(storage, sender.clone(), |x| -> Result<_, BalanceError> {
        if x.is_none() {
            return Ok(vec![]);
        } else {
            let result = x.unwrap().sub_balances_checked(&to_add);

            if result.is_ok() {
                return Ok(result.unwrap());
            } else {
                return Ok(vec![]);
            }
        }
    })?;

    TOTAL_BALANCE.update(storage, |x| -> Result<_, BalanceError> {
        Ok(x.add_balances_checked(&to_add)?)
    })?;

    Ok(Response::new())
}

pub fn handle_competition_result(
    mut deps: DepsMut,
    info: MessageInfo,
    cw_competition_result_msg: CwCompetitionResultMsg,
) -> Result<Response, ContractError> {
    if !ADMIN.is_admin(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }
    if STATE.may_load(deps.storage)? != Some(EscrowState::Locked {}) {
        return Err(ContractError::InvalidState {});
    }

    let response = match cw_competition_result_msg.distribution {
        None => {
            let addrs = BALANCE
                .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
                .collect::<StdResult<Vec<Addr>>>()?;

            inner_refund(deps.branch(), addrs)?
        }
        Some(members) => {
            let total = TOTAL_BALANCE.load(deps.storage)?;
            let key = KEY.load(deps.storage)?;

            let msgs = disburse(
                deps.as_ref(),
                &total,
                info.sender,
                Some(members),
                Some(key.to_string()),
            )?;

            Response::new()
                .add_attribute("key", key.to_string())
                .add_messages(msgs)
        }
    };

    clear_state(deps);
    Ok(response.add_attribute("action", "handle_competition_result"))
}

pub fn handle_competition_state_changed(
    deps: DepsMut,
    info: MessageInfo,
    cw_competition_state_changed_msg: CwCompetitionStateChangedMsg,
) -> Result<Response, ContractError> {
    if !ADMIN.is_admin(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }
    let state = match cw_competition_state_changed_msg.new_state {
        CompetitionState::Pending => EscrowState::Unlocked {},
        CompetitionState::Staged => EscrowState::Locked {},
        CompetitionState::Active => EscrowState::Locked {},
        CompetitionState::Inactive => EscrowState::Unlocked {},
    };

    STATE.save(deps.storage, &state)?;
    Ok(Response::new()
        .add_attribute("action", "handle_competition_state_changed")
        .add_attribute(
            "new_state",
            cw_competition_state_changed_msg.new_state.as_str(),
        ))
}

pub fn lock(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    if !ADMIN.is_admin(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }
    STATE.save(deps.storage, &EscrowState::Locked {})?;
    Ok(Response::new().add_attribute("action", "lock"))
}

pub fn unlock(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    if !ADMIN.is_admin(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }
    STATE.save(deps.storage, &EscrowState::Unlocked {})?;
    Ok(Response::new().add_attribute("action", "unlock"))
}

fn clear_state(deps: DepsMut) {
    TOTAL_BALANCE.remove(deps.storage);
    BALANCE.clear(deps.storage);
    DUE.clear(deps.storage);
    STATE.remove(deps.storage);
}
