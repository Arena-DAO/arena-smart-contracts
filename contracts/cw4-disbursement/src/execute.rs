use cosmwasm_std::{from_binary, Addr, Coin, Deps, DepsMut, MessageInfo, Response, Uint128};
use cw20::Cw20ReceiveMsg;
use cw4_group::state::{ADMIN, MEMBERS};
use cw721::Cw721ReceiveMsg;
use cw_controllers::AdminError;
use cw_disbursement::disburse;
use cw_disbursement::{DisbursementData, MemberShare};
use cw_tokens::{BatchCoinExtensions, GenericTokenBalance, TokenExtensions};

use crate::state::DAO;
use crate::{state::DISBURSEMENT_DATA, ContractError};

fn is_authorized(deps: Deps, addr: &Addr) -> Result<bool, AdminError> {
    Ok(ADMIN.is_admin(deps, addr)? || DAO.load(deps.storage)? == addr.clone())
}

pub fn update_admin(
    deps: DepsMut,
    info: MessageInfo,
    admin: Option<String>,
) -> Result<Response, ContractError> {
    if !is_authorized(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }
    let api = deps.api;
    Ok(ADMIN.execute_update_admin(deps, info, Some(api.addr_validate(&admin.unwrap())?))?)
}

pub fn receive_native(
    deps: DepsMut,
    info: MessageInfo,
    disbursement_key: Option<String>,
) -> Result<Response, ContractError> {
    Ok(
        receive_generic_tokens(deps, &info.funds, &mut vec![], disbursement_key)?
            .add_attribute("action", "disburse"),
    )
}

pub fn cw721_receive(
    deps: DepsMut,
    info: MessageInfo,
    cw721_receive_msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    let key: Option<String> = from_binary(&cw721_receive_msg.msg)?;

    Ok(receive_generic_tokens(
        deps,
        &info.funds,
        &mut vec![cw721_receive_msg.to_generic(&info.sender)],
        key,
    )?
    .add_attribute("action", "cw721_receive")
    .add_attribute("cw721", &info.sender)
    .add_attribute("sender", &cw721_receive_msg.sender))
}

pub fn cw20_receive(
    deps: DepsMut,
    info: MessageInfo,
    cw20_receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let key: Option<String> = from_binary(&cw20_receive_msg.msg)?;

    Ok(receive_generic_tokens(
        deps,
        &info.funds,
        &mut vec![cw20_receive_msg.to_generic(&info.sender)],
        key,
    )?
    .add_attribute("action", "cw20_receive")
    .add_attribute("cw20", &info.sender)
    .add_attribute("sender", &cw20_receive_msg.sender))
}

fn receive_generic_tokens(
    deps: DepsMut,
    funds: &Vec<Coin>,
    tokens: &mut Vec<GenericTokenBalance>,
    key: Option<String>,
) -> Result<Response, ContractError> {
    //make sure we are considering native + token transfers
    tokens.append(&mut funds.to_generic_batch());

    //load the dao
    let dao = DAO.load(deps.storage)?;

    //load the disbursement data
    let disbursement_data = match &key {
        Some(val) => DISBURSEMENT_DATA.may_load(deps.storage, val.clone())?,
        None => None,
    };

    let msgs = disburse(
        deps.as_ref(),
        tokens,
        dao.clone(),
        disbursement_data.map(|x| x.members),
        key.clone(),
    )?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("disbursement_key", key.unwrap_or("undefined".to_string()))
        .add_attribute("dao", &dao))
}

pub fn set_disbursement_data(
    deps: DepsMut,
    info: MessageInfo,
    key: String,
    disbursement_data: Vec<MemberShare>,
) -> Result<Response, ContractError> {
    //only the dao can change disbursement data
    if DAO.load(deps.storage)? != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let mut total_shares = Uint128::zero();
    let mut members = vec![];

    for member in disbursement_data {
        if member.shares.is_zero() {
            continue; //do not store zero values
        }

        let addr = deps.api.addr_validate(&member.addr)?;

        //do not allow mapping to users not in the group
        if MEMBERS.may_load(deps.storage, &addr)? == None {
            return Err(ContractError::MemberNotFound {});
        }

        total_shares = total_shares.checked_add(Uint128::from(member.shares))?;
        members.push(MemberShare {
            addr: member.addr,
            shares: member.shares,
        })
    }

    if total_shares.is_zero() {
        return Err(ContractError::ZeroSharesTotal {});
    }

    DISBURSEMENT_DATA.save(
        deps.storage,
        key.clone(),
        &DisbursementData {
            total_shares,
            members,
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "set_disbursement_data")
        .add_attribute("key", key))
}
