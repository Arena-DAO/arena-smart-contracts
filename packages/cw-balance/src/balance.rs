use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, OverflowError,
    OverflowOperation, StdResult, Uint128, WasmMsg,
};
use cw20::{Cw20Coin, Cw20CoinVerified, Cw20ExecuteMsg};
use cw721::Cw721ExecuteMsg;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::{is_contract, BalanceError, Cw721Collection, Cw721CollectionVerified, MemberShare};

// Struct to hold the verified member balance
#[cw_serde]
pub struct MemberBalanceChecked {
    pub addr: Addr,
    pub balance: BalanceVerified,
}

// Struct to hold the member balance
#[cw_serde]
pub struct MemberBalanceUnchecked {
    pub addr: String,
    pub balance: BalanceUnchecked,
}

// Method to convert MemberBalance to MemberBalanceVerified
impl MemberBalanceUnchecked {
    pub fn into_checked(self, deps: Deps) -> StdResult<MemberBalanceChecked> {
        Ok(MemberBalanceChecked {
            addr: deps.api.addr_validate(&self.addr)?,
            balance: self.balance.into_checked(deps)?,
        })
    }
}

// Struct to hold the balance
#[cw_serde]
pub struct BalanceUnchecked {
    pub native: Vec<Coin>,
    pub cw20: Vec<Cw20Coin>,
    pub cw721: Vec<Cw721Collection>,
}

// Method to convert Balance to BalanceVerified
impl BalanceUnchecked {
    pub fn into_checked(self, deps: Deps) -> StdResult<BalanceVerified> {
        Ok(BalanceVerified {
            native: self.native,
            cw20: self
                .cw20
                .iter()
                .map(|x| {
                    Ok(Cw20CoinVerified {
                        address: deps.api.addr_validate(&x.address)?,
                        amount: x.amount,
                    })
                })
                .collect::<StdResult<Vec<Cw20CoinVerified>>>()?,
            cw721: self
                .cw721
                .iter()
                .map(|x| {
                    Ok(Cw721CollectionVerified {
                        address: deps.api.addr_validate(&x.address)?,
                        token_ids: x.token_ids.clone(),
                    })
                })
                .collect::<StdResult<Vec<Cw721CollectionVerified>>>()?,
        })
    }
}

// Struct to hold the verified balance
#[cw_serde]
#[derive(Default)]
pub struct BalanceVerified {
    pub native: Vec<Coin>,
    pub cw20: Vec<Cw20CoinVerified>,
    pub cw721: Vec<Cw721CollectionVerified>,
}

// Display implementation for BalanceVerified
impl Display for BalanceVerified {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "Native: ")?;
        for coin in &self.native {
            writeln!(f, "  {}", coin)?;
        }

        writeln!(f, "CW20:")?;
        for cw20_coin in &self.cw20 {
            writeln!(f, "  {}", cw20_coin)?;
        }

        writeln!(f, "CW721:")?;
        for cw721_tokens in &self.cw721 {
            writeln!(f, "  {}", cw721_tokens)?;
        }

        Ok(())
    }
}

// Methods for BalanceVerified
impl BalanceVerified {
    // Method to create a new BalanceVerified
    pub fn new() -> Self {
        Self::default()
    }

    // Method to calculate the amounts needed to reach other balance
    pub fn difference(&self, other: &BalanceVerified) -> StdResult<BalanceVerified> {
        let mut diff = BalanceVerified::new();

        let native_map: HashMap<&String, Uint128> = self
            .native
            .iter()
            .map(|coin| (&coin.denom, coin.amount))
            .collect();

        for coin in &other.native {
            match native_map.get(&coin.denom) {
                Some(&amount) if amount < coin.amount => {
                    diff.native.push(Coin {
                        denom: coin.denom.clone(),
                        amount: coin.amount.checked_sub(amount)?,
                    });
                }
                None => diff.native.push(coin.clone()),
                _ => (),
            }
        }

        let cw20_map: HashMap<&Addr, Uint128> = self
            .cw20
            .iter()
            .map(|coin| (&coin.address, coin.amount))
            .collect();

        for coin in &other.cw20 {
            match cw20_map.get(&coin.address) {
                Some(&amount) if amount < coin.amount => {
                    diff.cw20.push(Cw20CoinVerified {
                        address: coin.address.clone(),
                        amount: coin.amount.checked_sub(amount)?,
                    });
                }
                None => diff.cw20.push(coin.clone()),
                _ => (),
            }
        }

        let cw721_map: HashMap<&Addr, BTreeSet<&String>> = self
            .cw721
            .iter()
            .map(|token| (&token.address, token.token_ids.iter().collect()))
            .collect();

        for token in &other.cw721 {
            let token_ids_set: BTreeSet<&String> = token.token_ids.iter().collect();
            match cw721_map.get(&token.address) {
                Some(token_ids)
                    if token_ids_set.is_superset(token_ids)
                        && token_ids_set.len() != token_ids.len() =>
                {
                    let diff_token_ids: Vec<String> = token_ids_set
                        .difference(token_ids)
                        .cloned()
                        .map(|s| s.to_owned())
                        .collect();
                    diff.cw721.push(Cw721CollectionVerified {
                        address: token.address.clone(),
                        token_ids: diff_token_ids,
                    });
                }
                None => diff.cw721.push(token.clone()),
                _ => (),
            }
        }

        Ok(diff)
    }

    // Method to check if BalanceVerified is empty
    pub fn is_empty(&self) -> bool {
        self.native.is_empty() && self.cw20.is_empty() && self.cw721.is_empty()
    }

    // Method to add two BalanceVerified together
    pub fn checked_add(&self, other: &BalanceVerified) -> StdResult<BalanceVerified> {
        if self.is_empty() {
            return Ok(other.clone());
        }
        if other.is_empty() {
            return Ok(self.clone());
        }

        let mut native_map: BTreeMap<&String, Uint128> = self
            .native
            .iter()
            .map(|coin| (&coin.denom, coin.amount))
            .collect();
        for coin in &other.native {
            let entry = native_map.entry(&coin.denom).or_default();
            *entry = entry.checked_add(coin.amount)?;
        }

        let mut cw20_map: BTreeMap<&Addr, Uint128> = self
            .cw20
            .iter()
            .map(|coin| (&coin.address, coin.amount))
            .collect();
        for coin in &other.cw20 {
            let entry = cw20_map.entry(&coin.address).or_insert(Uint128::zero());
            *entry = entry.checked_add(coin.amount)?;
        }

        let mut cw721_map: BTreeMap<&Addr, BTreeSet<&String>> = self
            .cw721
            .iter()
            .map(|token| (&token.address, token.token_ids.iter().collect()))
            .collect();

        for token in &other.cw721 {
            let entry = cw721_map.entry(&token.address).or_default();

            for token_id in &token.token_ids {
                // If the token_id is already present, it's a duplicate and we return an error.
                if !entry.insert(token_id) {
                    return Err(cosmwasm_std::StdError::Overflow {
                        source: OverflowError::new(OverflowOperation::Add, self, other),
                    });
                }
            }
        }

        Ok(BalanceVerified {
            native: native_map
                .into_iter()
                .map(|(denom, amount)| Coin {
                    denom: denom.to_string(),
                    amount,
                })
                .collect(),
            cw20: cw20_map
                .into_iter()
                .map(|(address, amount)| Cw20CoinVerified {
                    address: address.clone(),
                    amount,
                })
                .collect(),
            cw721: cw721_map
                .into_iter()
                .map(|(addr, token_ids)| Cw721CollectionVerified {
                    address: addr.clone(),
                    token_ids: token_ids
                        .into_iter()
                        .map(|token| token.to_string())
                        .collect(),
                })
                .collect(),
        })
    }

    // Method to subtract one BalanceVerified from another
    pub fn checked_sub(&self, other: &BalanceVerified) -> StdResult<BalanceVerified> {
        if other.is_empty() {
            return Ok(self.clone());
        }

        let mut native_map: BTreeMap<&String, Uint128> = self
            .native
            .iter()
            .map(|coin| (&coin.denom, coin.amount))
            .collect();
        for coin in &other.native {
            match native_map.entry(&coin.denom) {
                Entry::Occupied(mut entry) => {
                    let total_amount = entry.get_mut().checked_sub(coin.amount)?;
                    if total_amount.is_zero() {
                        entry.remove();
                    } else {
                        *entry.get_mut() = total_amount;
                    }
                }

                Entry::Vacant(_) => {
                    return Err(cosmwasm_std::StdError::Overflow {
                        source: cosmwasm_std::OverflowError::new(
                            OverflowOperation::Sub,
                            self,
                            other,
                        ),
                    });
                }
            }
        }

        let mut cw20_map: BTreeMap<&Addr, Uint128> = self
            .cw20
            .iter()
            .map(|coin| (&coin.address, coin.amount))
            .collect();
        for coin in &other.cw20 {
            match cw20_map.entry(&coin.address) {
                Entry::Occupied(mut entry) => {
                    let total_amount = entry.get_mut().checked_sub(coin.amount)?;
                    if total_amount.is_zero() {
                        entry.remove();
                    } else {
                        *entry.get_mut() = total_amount;
                    }
                }

                Entry::Vacant(_) => {
                    return Err(cosmwasm_std::StdError::Overflow {
                        source: cosmwasm_std::OverflowError::new(
                            OverflowOperation::Sub,
                            self,
                            other,
                        ),
                    });
                }
            }
        }

        let mut cw721_map: BTreeMap<&Addr, BTreeSet<&String>> = self
            .cw721
            .iter()
            .map(|token| (&token.address, token.token_ids.iter().collect()))
            .collect();
        for token in &other.cw721 {
            if let Some(entry_set) = cw721_map.get_mut(&token.address) {
                for token_id in &token.token_ids {
                    // Removes the token_id from the set if it exists; no-op if it doesn't
                    if !entry_set.remove(token_id) {
                        // Return error if a token_id is missing
                        return Err(cosmwasm_std::StdError::Overflow {
                            source: OverflowError::new(OverflowOperation::Sub, self, other),
                        });
                    }
                }

                if entry_set.is_empty() {
                    cw721_map.remove(&token.address);
                }
            } else {
                // Return error if a corresponding addr is missing
                return Err(cosmwasm_std::StdError::Overflow {
                    source: OverflowError::new(OverflowOperation::Sub, self, other),
                });
            }
        }

        Ok(BalanceVerified {
            native: native_map
                .into_iter()
                .map(|(denom, amount)| Coin {
                    denom: denom.to_string(),
                    amount,
                })
                .collect(),
            cw20: cw20_map
                .into_iter()
                .map(|(address, amount)| Cw20CoinVerified {
                    address: address.clone(),
                    amount,
                })
                .collect(),
            cw721: cw721_map
                .into_iter()
                .map(|(addr, tokens)| Cw721CollectionVerified {
                    address: addr.clone(),
                    token_ids: tokens
                        .into_iter()
                        .map(|token_id| token_id.to_string())
                        .collect(),
                })
                .collect(),
        })
    }

    // Method to transmit all types of tokens (native, CW20, CW721) to a recipient
    pub fn transmit_all(
        &self,
        deps: Deps,
        recipient: &Addr,
        cw20_msg: Option<Binary>,
        cw721_msg: Option<Binary>,
    ) -> StdResult<Vec<CosmosMsg>> {
        match is_contract(deps, recipient.to_string()) {
            false => self.transfer_all(recipient),
            true => self.send_all(recipient, cw20_msg, cw721_msg),
        }
    }

    // Method to transfer all types of tokens (native, CW20, CW721) to a recipient
    pub fn transfer_all(&self, recipient: &Addr) -> StdResult<Vec<CosmosMsg>> {
        let mut messages: Vec<CosmosMsg> = Vec::new();

        // Transfer native tokens
        messages.extend(self.send_native(recipient.to_string()));

        // Transfer CW20 tokens
        messages.extend(self.transfer_cw20(recipient.to_string())?);

        // Transfer CW721 tokens
        messages.extend(self.transfer_cw721(recipient.to_string())?);

        Ok(messages)
    }

    // Method to send all types of tokens (native, CW20, CW721) to a contract
    pub fn send_all(
        &self,
        contract_addr: &Addr,
        cw20_msg: Option<Binary>,
        cw721_msg: Option<Binary>,
    ) -> StdResult<Vec<CosmosMsg>> {
        let mut messages: Vec<CosmosMsg> = Vec::new();

        // Send native tokens to contract
        messages.extend(self.send_native(contract_addr.to_string()));

        // Send CW20 tokens to contract
        messages.extend(self.send_cw20(contract_addr.to_string(), cw20_msg.unwrap_or_default())?);

        // Send CW721 tokens to contract
        messages.extend(self.send_cw721(contract_addr.to_string(), cw721_msg.unwrap_or_default())?);

        Ok(messages)
    }

    // Method to send native tokens to an address
    pub fn send_native(&self, to_address: String) -> Vec<CosmosMsg> {
        if self.native.is_empty() {
            vec![]
        } else {
            vec![CosmosMsg::Bank(BankMsg::Send {
                to_address,
                amount: self.native.clone(),
            })]
        }
    }

    // Method to send CW20 tokens to a contract
    pub fn send_cw20(&self, contract: String, msg: Binary) -> StdResult<Vec<CosmosMsg>> {
        self.cw20
            .iter()
            .map(|cw20_coin| {
                let exec_msg = WasmMsg::Execute {
                    contract_addr: cw20_coin.address.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Send {
                        contract: contract.clone(),
                        amount: cw20_coin.amount,
                        msg: msg.clone(),
                    })?,
                    funds: vec![],
                };
                Ok(CosmosMsg::Wasm(exec_msg))
            })
            .collect()
    }

    // Method to send CW721 tokens to a contract
    pub fn send_cw721(&self, contract: String, msg: Binary) -> StdResult<Vec<CosmosMsg>> {
        self.cw721
            .iter()
            .flat_map(|cw721_collection| {
                let contract = contract.clone();
                let msg = msg.clone();
                cw721_collection.token_ids.iter().map(move |token_id| {
                    let exec_msg = WasmMsg::Execute {
                        contract_addr: cw721_collection.address.to_string(),
                        msg: to_json_binary(&Cw721ExecuteMsg::SendNft {
                            contract: contract.clone(),
                            token_id: token_id.clone(),
                            msg: msg.clone(),
                        })?,
                        funds: vec![],
                    };
                    Ok(CosmosMsg::Wasm(exec_msg))
                })
            })
            .collect()
    }

    // Method to transfer CW721 tokens to a recipient
    pub fn transfer_cw721(&self, recipient: String) -> StdResult<Vec<CosmosMsg>> {
        self.cw721
            .iter()
            .flat_map(|cw721_collection| {
                let recipient = recipient.clone();
                cw721_collection.token_ids.iter().map(move |token_id| {
                    let exec_msg = WasmMsg::Execute {
                        contract_addr: cw721_collection.address.to_string(),
                        msg: to_json_binary(&Cw721ExecuteMsg::TransferNft {
                            recipient: recipient.to_string(),
                            token_id: token_id.clone(),
                        })?,
                        funds: vec![],
                    };
                    Ok(CosmosMsg::Wasm(exec_msg))
                })
            })
            .collect()
    }

    // Method to transfer CW20 tokens to a recipient
    pub fn transfer_cw20(&self, recipient: String) -> StdResult<Vec<CosmosMsg>> {
        self.cw20
            .iter()
            .map(|cw20_coin| {
                let exec_msg = WasmMsg::Execute {
                    contract_addr: cw20_coin.address.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: recipient.clone(),
                        amount: cw20_coin.amount,
                    })?,
                    funds: vec![],
                };
                Ok(CosmosMsg::Wasm(exec_msg))
            })
            .collect()
    }

    // Method to split the balance among multiple users based on their assigned weights
    pub fn split(
        &self,
        distribution: &Vec<MemberShare<Addr>>,
        remainder_address: &Addr,
    ) -> Result<Vec<MemberBalanceChecked>, BalanceError> {
        let total_weight = distribution
            .iter()
            .try_fold(Uint128::zero(), |accumulator, x| {
                accumulator.checked_add(x.shares)
            })?;
        let mut split_balances: Vec<MemberBalanceChecked> = Vec::new();

        let mut remainders_native: BTreeMap<String, Uint128> = self
            .native
            .iter()
            .map(|x| (x.denom.clone(), x.amount))
            .collect();
        let mut remainders_cw20: BTreeMap<Addr, Uint128> = self
            .cw20
            .iter()
            .map(|x| (x.address.clone(), x.amount))
            .collect();

        for member_share in distribution {
            let weight_fraction = Decimal::from_ratio(member_share.shares, total_weight);

            let mut split_native = BTreeMap::new();
            for coin in &self.native {
                let decimal_amount = Decimal::from_atomics(coin.amount, 0u32)?;
                let split_amount = weight_fraction.checked_mul(decimal_amount)?.to_uint_floor();

                // Deduct the split amount from the remainder
                if let Some(remainder) = remainders_native.get_mut(&coin.denom) {
                    *remainder = remainder.checked_sub(split_amount)?;
                }

                split_native.insert(coin.denom.clone(), split_amount);
            }

            let mut split_cw20 = BTreeMap::new();
            for cw20_coin in &self.cw20 {
                let decimal_amount = Decimal::from_atomics(cw20_coin.amount, 0u32)?;
                let split_amount = weight_fraction.checked_mul(decimal_amount)?.to_uint_floor();

                // Deduct the split amount from the remainder
                if let Some(remainder) = remainders_cw20.get_mut(&cw20_coin.address) {
                    *remainder = remainder.checked_sub(split_amount)?;
                }

                split_cw20.insert(cw20_coin.address.clone(), split_amount);
            }

            let split_balance = BalanceVerified {
                native: split_native
                    .into_iter()
                    .map(|(denom, amount)| Coin { denom, amount })
                    .collect(),
                cw20: split_cw20
                    .into_iter()
                    .map(|(address, amount)| Cw20CoinVerified { address, amount })
                    .collect(),
                cw721: vec![],
            };

            let member_balance = MemberBalanceChecked {
                addr: member_share.addr.clone(),
                balance: split_balance,
            };

            split_balances.push(member_balance);
        }

        // Apply the remainder_balance to the corresponding split_balances entry
        let remainder_balance = BalanceVerified {
            native: remainders_native
                .into_iter()
                .map(|(denom, amount)| Coin { denom, amount })
                .collect(),
            cw20: remainders_cw20
                .into_iter()
                .map(|(address, amount)| Cw20CoinVerified { address, amount })
                .collect(),
            cw721: self.cw721.clone(),
        };

        if !remainder_balance.is_empty() {
            if let Some(member_balance) = split_balances
                .iter_mut()
                .find(|mb| mb.addr == remainder_address)
            {
                member_balance.balance = member_balance.balance.checked_add(&remainder_balance)?;
            } else {
                split_balances.push(MemberBalanceChecked {
                    addr: remainder_address.clone(),
                    balance: remainder_balance,
                });
            }
        }

        Ok(split_balances)
    }
}
