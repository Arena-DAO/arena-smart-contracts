use cosmwasm_std::{
    Attribute, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    Uint128,
};
use cw_utils::must_pay;

use crate::{
    query,
    state::{FundraiseState, CONFIG, TOTAL_DEPOSITED, USER_DEPOSIT},
    ContractError,
};

pub fn deposit(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if config.state != FundraiseState::Active {
        return Err(ContractError::FundraiseNotActive {});
    }

    if let Some(start) = config.start {
        if !start.is_expired(&env.block) {
            return Err(ContractError::StdError(StdError::generic_err(
                "The fundraise start has not begun",
            )));
        }
    }

    if config.end.is_expired(&env.block) {
        return Err(ContractError::StdError(StdError::generic_err(
            "The fundraise has already ended",
        )));
    }

    let deposit_amount = must_pay(&info, &config.deposit_denom)?;

    let balance = USER_DEPOSIT.update(
        deps.storage,
        &info.sender,
        |user_deposit| -> StdResult<Uint128> {
            match user_deposit {
                None => Ok(deposit_amount),
                Some(balance) => Ok(balance.checked_add(deposit_amount)?),
            }
        },
    )?;

    let total_deposited =
        TOTAL_DEPOSITED.update(deps.storage, |total_deposited| -> StdResult<Uint128> {
            Ok(total_deposited.checked_add(deposit_amount)?)
        })?;

    Ok(Response::new()
        .add_attribute("action", "deposit")
        .add_attribute("balance", balance)
        .add_attribute("total_deposited", total_deposited))
}

pub fn withdraw(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    if !USER_DEPOSIT.has(deps.storage, &info.sender) {
        return Err(ContractError::StdError(StdError::generic_err(
            "User has no balance",
        )));
    }

    let config = CONFIG.load(deps.storage)?;

    let mut attrs = vec![];
    let msg = match config.state {
        FundraiseState::Active => {
            let balance = USER_DEPOSIT.load(deps.storage, &info.sender)?;
            let total_deposited = TOTAL_DEPOSITED
                .update(deps.storage, |total_deposited| -> StdResult<_> {
                    Ok(total_deposited.checked_sub(balance)?)
                })?;

            attrs.push(Attribute::new("total_deposited", total_deposited));
            CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![Coin {
                    amount: balance,
                    denom: config.deposit_denom,
                }],
            })
        }
        FundraiseState::Failed => {
            let balance = USER_DEPOSIT.load(deps.storage, &info.sender)?;

            CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![Coin {
                    amount: balance,
                    denom: config.deposit_denom,
                }],
            })
        }
        FundraiseState::Successful => {
            // Can safely call unwrap here, because it will have a value if USER_DEPOSIT is checked for a value
            let reward = query::reward(deps.as_ref(), info.sender.to_string())?.unwrap();

            attrs.push(Attribute::new("reward", reward));
            CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![Coin {
                    denom: config.fundraise.denom,
                    amount: reward,
                }],
            })
        }
    };

    USER_DEPOSIT.remove(deps.storage, &info.sender);

    Ok(Response::new()
        .add_attribute("action", "withdraw")
        .add_attributes(attrs)
        .add_message(msg))
}

pub fn expire(deps: DepsMut, env: Env, _info: MessageInfo) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if config.state != FundraiseState::Active {
        return Err(ContractError::FundraiseNotActive {});
    }
    if !config.end.is_expired(&env.block) {
        return Err(ContractError::StdError(StdError::generic_err(
            "The fundraise has not expired yet",
        )));
    }

    let total_deposited = TOTAL_DEPOSITED.load(deps.storage)?;

    let mut msgs = vec![];
    if total_deposited > config.soft_cap {
        config.state = FundraiseState::Successful;

        // Send the deposits to the recipient
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: config.recipient.to_string(),
            amount: vec![Coin {
                amount: total_deposited,
                denom: config.deposit_denom.clone(),
            }],
        }))
    } else {
        config.state = FundraiseState::Failed;

        // Send the fundraise amount back to the recipient
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: config.recipient.to_string(),
            amount: vec![config.fundraise.clone()],
        }))
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "expire")
        .add_attribute("outcome", config.state.to_string())
        .add_messages(msgs))
}
