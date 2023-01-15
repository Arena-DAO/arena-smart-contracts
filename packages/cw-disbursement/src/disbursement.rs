use std::collections::{HashMap, HashSet};

use cosmwasm_std::{to_binary, Addr, BankMsg, Coin, CosmosMsg, Deps, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use cw721::Cw721ExecuteMsg;
use cw_tokens::{GenericTokenBalance, GenericTokenType};

use crate::{
    error::DisbursementError, models::MemberBalance, CwDisbursementContract,
    CwDisbursementExecuteMsg, MemberShare,
};

pub fn disburse(
    deps: Deps,
    tokens: &Vec<GenericTokenBalance>,
    underflow_addr: Addr,
    shares: Option<Vec<MemberShare>>,
    key: Option<String>,
) -> Result<Vec<CosmosMsg>, DisbursementError> {
    let mut msgs = vec![];

    //initialize the contracts list
    let contract = CwDisbursementContract(underflow_addr.clone());
    let mut contracts: HashSet<Addr> = HashSet::default();
    if contract.is_disbursement_contract(&deps.querier, &key) {
        contracts.insert(underflow_addr.clone());
    }

    //initialize the distribution list
    let mut member_balances: HashMap<Addr, Vec<GenericTokenBalance>> = HashMap::new();

    if shares.is_none() {
        member_balances.insert(contract.0, tokens.to_vec());
    } else {
        let shares = shares.unwrap();

        //calculate total_shares
        let mut total_shares = Uint128::zero();
        for share in &shares {
            total_shares = total_shares.checked_add(share.shares)?;
        }
        if total_shares.is_zero() {
            return Err(DisbursementError::InvalidShares {});
        }

        let mut is_contracts_set = false;

        //create a distribution set
        for token in tokens {
            //if we cannot disburse fairly, then send it all to the admin (dao) for handling
            if Uint128::from(shares.len() as u128) > token.amount {
                if member_balances.contains_key(&underflow_addr) {
                    member_balances
                        .get_mut(&underflow_addr)
                        .unwrap()
                        .push(token.clone());
                } else {
                    member_balances.insert(underflow_addr.clone(), vec![token.clone()]);
                }
                continue;
            }

            //disburse the tokens into the members
            let mut total = token.amount;
            for (i, member) in shares.iter().enumerate() {
                //calculate amounts
                let mut amount = token
                    .amount
                    .checked_mul(member.shares)?
                    .checked_div(total_shares)?;
                total = total.checked_sub(amount)?;

                //give the last member the remainder to avoid executing extra messages
                if i == shares.len() - 1 && !total.is_zero() {
                    amount = amount.checked_add(total)?;
                }

                //construct a contracts set
                let addr = deps.api.addr_validate(&member.addr)?;
                let contract = CwDisbursementContract(addr.clone());
                if !is_contracts_set && contract.is_disbursement_contract(&deps.querier, &key) {
                    contracts.insert(addr.clone());
                }

                //create the transfer logic
                if member_balances.contains_key(&addr) {
                    member_balances
                        .get_mut(&addr)
                        .unwrap()
                        .push(token.clone_with_amount(amount));
                } else {
                    member_balances.insert(addr.clone(), vec![token.clone_with_amount(amount)]);
                }
            }

            is_contracts_set = true; //after the first set of iterations, we know which members are contracts
        }
    }

    let member_balances: Vec<MemberBalance> = member_balances
        .iter()
        .map(|x| MemberBalance {
            member: x.0.to_string(),
            balances: x.1.to_vec(),
        })
        .collect();
    for member_balance in member_balances {
        msgs.append(&mut member_balance.to_msgs(key.clone(), &contracts)?);
    }

    Ok(msgs)
}

impl MemberBalance {
    pub fn to_msgs(
        &self,
        key: Option<String>,
        contracts: &HashSet<Addr>,
    ) -> Result<Vec<CosmosMsg>, DisbursementError> {
        let mut msgs: Vec<CosmosMsg> = vec![];
        let mut coins_map: HashMap<String, Vec<Coin>> = HashMap::new();

        for balance in &self.balances {
            match balance.token_type {
                GenericTokenType::Native => {
                    if balance.denom.is_none() || balance.addr.is_some() {
                        return Err(DisbursementError::InvalidToken {});
                    }
                    let coin = Coin {
                        denom: balance.denom.as_ref().unwrap().to_string(),
                        amount: balance.amount,
                    };

                    //batch this
                    if coins_map.contains_key(&self.member) {
                        coins_map.get_mut(&self.member).unwrap().push(coin)
                    } else {
                        coins_map.insert(self.member.clone(), vec![coin]);
                    }
                }
                GenericTokenType::Cw20 => {
                    if balance.addr.is_none() || balance.denom.is_some() {
                        return Err(DisbursementError::InvalidToken {});
                    }

                    let msg = match contracts.contains(&Addr::unchecked(self.member.clone())) {
                        true => to_binary(&Cw20ExecuteMsg::Send {
                            contract: self.member.to_string(),
                            amount: balance.amount,
                            msg: to_binary(&key)?,
                        })?,
                        false => to_binary(&Cw20ExecuteMsg::Transfer {
                            recipient: self.member.to_string(),
                            amount: balance.amount,
                        })?,
                    };

                    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: balance.addr.as_ref().unwrap().to_string(),
                        msg,
                        funds: vec![],
                    }))
                }
                GenericTokenType::Cw721 => {
                    if balance.addr.is_none() || balance.denom.is_none() {
                        return Err(DisbursementError::InvalidToken {});
                    }

                    let msg = match contracts.contains(&Addr::unchecked(self.member.clone())) {
                        true => to_binary(&Cw721ExecuteMsg::SendNft {
                            contract: self.member.to_string(),
                            token_id: balance.denom.as_ref().unwrap().clone(),
                            msg: to_binary(&key)?,
                        })?,
                        false => to_binary(&Cw721ExecuteMsg::TransferNft {
                            recipient: self.member.to_string(),
                            token_id: balance.denom.as_ref().unwrap().clone(),
                        })?,
                    };

                    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: balance.addr.as_ref().unwrap().to_string(),
                        msg,
                        funds: vec![],
                    }))
                }
            };
        }

        //batch send native coins
        for member_coins in coins_map {
            //signal the contract to disburse if it's a contract
            if contracts.contains(&Addr::unchecked(member_coins.0.clone())) {
                msgs.push(
                    CwDisbursementExecuteMsg::ReceiveNative { key: key.clone() }
                        .into_cosmos_msg(member_coins.0, member_coins.1)?,
                )
            } else {
                msgs.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: member_coins.0.to_string(),
                    amount: member_coins.1,
                }));
            }
        }

        Ok(msgs)
    }
}
