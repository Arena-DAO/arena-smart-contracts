use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, OverflowError,
    OverflowOperation, StdResult, Uint128, WasmMsg,
};
use cw20::{Cw20CoinVerified, Cw20ExecuteMsg};
use cw721::Cw721ExecuteMsg;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::{is_contract, BalanceError, Cw721TokensVerified, MemberShareValidated};

#[cw_serde]
pub struct MemberBalance {
    pub addr: Addr,
    pub balance: Balance,
}

#[cw_serde]
pub struct Balance {
    pub native: Vec<Coin>,
    pub cw20: Vec<Cw20CoinVerified>,
    pub cw721: Vec<Cw721TokensVerified>,
}

impl Default for Balance {
    fn default() -> Self {
        Self {
            native: vec![],
            cw20: vec![],
            cw721: vec![],
        }
    }
}

impl Display for Balance {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "Native:")?;
        for coin in &self.native {
            writeln!(f, "  {}: {}", coin.denom, coin.amount)?;
        }

        writeln!(f, "CW20:")?;
        for cw20_coin in &self.cw20 {
            writeln!(f, "  {}: {}", cw20_coin.address, cw20_coin.amount)?;
        }

        writeln!(f, "CW721:")?;
        for cw721_tokens in &self.cw721 {
            writeln!(f, "  {}: {:?}", cw721_tokens.addr, cw721_tokens.token_ids)?;
        }

        Ok(())
    }
}

impl Balance {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.native.is_empty() && self.cw20.is_empty() && self.cw721.is_empty()
    }

    pub fn checked_add(&self, other: &Balance) -> StdResult<Balance> {
        let mut native_map: HashMap<String, Uint128> = self
            .native
            .iter()
            .map(|coin| (coin.denom.clone(), coin.amount))
            .collect();
        for coin in &other.native {
            let total_amount = native_map
                .entry(coin.denom.clone())
                .or_insert(Uint128::zero())
                .checked_add(coin.amount)?;
            native_map.insert(coin.denom.clone(), total_amount);
        }

        let mut cw20_map: HashMap<Addr, Uint128> = self
            .cw20
            .iter()
            .map(|coin| (coin.address.clone(), coin.amount))
            .collect();
        for coin in &other.cw20 {
            let total_amount = cw20_map
                .entry(coin.address.clone())
                .or_insert(Uint128::zero())
                .checked_add(coin.amount)?;
            cw20_map.insert(coin.address.clone(), total_amount);
        }

        let mut cw721_map: HashMap<Addr, Vec<String>> = self
            .cw721
            .iter()
            .map(|token| (token.addr.clone(), token.token_ids.clone()))
            .collect();
        for token in &other.cw721 {
            let entry = cw721_map.entry(token.addr.clone()).or_insert_with(Vec::new);
            entry.extend(token.token_ids.clone());
        }

        Ok(Balance {
            native: native_map
                .into_iter()
                .map(|(denom, amount)| Coin { denom, amount })
                .collect(),
            cw20: cw20_map
                .into_iter()
                .map(|(address, amount)| Cw20CoinVerified { address, amount })
                .collect(),
            cw721: cw721_map
                .into_iter()
                .map(|(addr, token_ids)| Cw721TokensVerified { addr, token_ids })
                .collect(),
        })
    }

    pub fn checked_sub(&self, other: &Balance) -> StdResult<Balance> {
        let mut native_map: HashMap<String, Uint128> = self
            .native
            .iter()
            .map(|coin| (coin.denom.clone(), coin.amount))
            .collect();
        for coin in &other.native {
            let total_amount = native_map
                .get_mut(&coin.denom)
                .unwrap_or(&mut Uint128::zero())
                .checked_sub(coin.amount)?;
            native_map.insert(coin.denom.clone(), total_amount);
        }

        let mut cw20_map: HashMap<Addr, Uint128> = self
            .cw20
            .iter()
            .map(|coin| (coin.address.clone(), coin.amount))
            .collect();
        for coin in &other.cw20 {
            let total_amount = cw20_map
                .get_mut(&coin.address)
                .unwrap_or(&mut Uint128::zero())
                .checked_sub(coin.amount)?;
            cw20_map.insert(coin.address.clone(), total_amount);
        }

        let mut cw721_map: HashMap<Addr, Vec<String>> = self
            .cw721
            .iter()
            .map(|token| (token.addr.clone(), token.token_ids.clone()))
            .collect();
        for token in &other.cw721 {
            let entry = cw721_map
                .get_mut(&token.addr)
                .ok_or_else(|| OverflowError::new(OverflowOperation::Sub, self, other))?;

            for token_id in &token.token_ids {
                entry
                    .iter()
                    .position(|x| x == token_id)
                    .ok_or_else(|| OverflowError::new(OverflowOperation::Sub, self, other))
                    .map(|i| entry.remove(i))?;
            }
        }

        Ok(Balance {
            native: native_map
                .into_iter()
                .map(|(denom, amount)| Coin { denom, amount })
                .collect(),
            cw20: cw20_map
                .into_iter()
                .map(|(address, amount)| Cw20CoinVerified { address, amount })
                .collect(),
            cw721: cw721_map
                .into_iter()
                .map(|(addr, tokens)| Cw721TokensVerified {
                    addr,
                    token_ids: tokens,
                })
                .collect(),
        })
    }

    pub fn transmit(
        &self,
        deps: Deps,
        recipient: &Addr,
        cw20_msg: Option<Binary>,
        cw721_msg: Option<Binary>,
    ) -> StdResult<Vec<CosmosMsg>> {
        match is_contract(deps, recipient.to_string()) {
            true => self.transfer(recipient),
            false => self.send(recipient, cw20_msg, cw721_msg),
        }
    }

    pub fn transfer(&self, recipient: &Addr) -> StdResult<Vec<CosmosMsg>> {
        let mut messages: Vec<CosmosMsg> = Vec::new();

        // Send native tokens
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient.to_string(),
            amount: self.native.clone(),
        }));

        // Send CW20 tokens
        for cw20_coin in &self.cw20 {
            let send_msg = Cw20ExecuteMsg::Transfer {
                recipient: recipient.to_string(),
                amount: cw20_coin.amount,
            };
            let exec_msg = WasmMsg::Execute {
                contract_addr: cw20_coin.address.to_string(),
                msg: to_binary(&send_msg)?,
                funds: vec![],
            };
            messages.push(CosmosMsg::Wasm(exec_msg));
        }

        // Send CW721 tokens
        for cw721_tokens in &self.cw721 {
            for token_id in &cw721_tokens.token_ids {
                let transfer_msg = Cw721ExecuteMsg::TransferNft {
                    recipient: recipient.to_string(),
                    token_id: token_id.clone(),
                };
                let exec_msg = WasmMsg::Execute {
                    contract_addr: cw721_tokens.addr.to_string(),
                    msg: to_binary(&transfer_msg)?,
                    funds: vec![],
                };
                messages.push(CosmosMsg::Wasm(exec_msg));
            }
        }

        Ok(messages)
    }

    pub fn send(
        &self,
        recipient: &Addr,
        cw20_msg: Option<Binary>,
        cw721_msg: Option<Binary>,
    ) -> StdResult<Vec<CosmosMsg>> {
        let mut messages: Vec<CosmosMsg> = Vec::new();

        // Send native tokens
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient.to_string(),
            amount: self.native.clone(),
        }));

        // Send CW20 tokens
        for cw20_coin in &self.cw20 {
            let send_msg = Cw20ExecuteMsg::Send {
                contract: recipient.to_string(),
                msg: cw20_msg.clone().unwrap_or(Binary::default()),
                amount: cw20_coin.amount,
            };
            let exec_msg = WasmMsg::Execute {
                contract_addr: cw20_coin.address.to_string(),
                msg: to_binary(&send_msg)?,
                funds: vec![],
            };
            messages.push(CosmosMsg::Wasm(exec_msg));
        }

        // Send CW721 tokens
        for cw721_tokens in &self.cw721 {
            for token_id in &cw721_tokens.token_ids {
                let transfer_msg = Cw721ExecuteMsg::SendNft {
                    contract: recipient.to_string(),
                    msg: cw721_msg.clone().unwrap_or(Binary::default()),
                    token_id: token_id.clone(),
                };
                let exec_msg = WasmMsg::Execute {
                    contract_addr: cw721_tokens.addr.to_string(),
                    msg: to_binary(&transfer_msg)?,
                    funds: vec![],
                };
                messages.push(CosmosMsg::Wasm(exec_msg));
            }
        }

        Ok(messages)
    }

    /// Splits a given `Balance` among multiple users based on their assigned weights.
    ///
    /// # Arguments
    ///
    /// * `weights` - A reference to a `HashMap` containing user addresses and their corresponding weights as `u128` values.
    /// * `remainder_address` - A reference to the address that will receive any remaining tokens after the split, as well as all NFTs.
    ///
    /// # Returns
    ///
    /// A `StdResult` containing a `HashMap` of user addresses mapped to their respective split `Balance` instances.
    ///
    /// # Errors
    ///
    /// This function may return an error if any of the following occurs:
    ///
    /// * Division by zero when calculating the weight fraction.
    /// * Multiplication overflow when calculating the split amounts for native and CW20 tokens.
    /// * Subtraction underflow when updating the remainders for native and CW20 tokens.
    pub fn split(
        &self,
        distribution: &Vec<MemberShareValidated>,
        remainder_address: &Addr,
    ) -> Result<Vec<MemberBalance>, BalanceError> {
        let total_weight = distribution
            .iter()
            .try_fold(Uint128::zero(), |accumulator, x| {
                accumulator.checked_add(x.shares)
            })?;
        let mut split_balances: Vec<MemberBalance> = Vec::new();

        let mut remainders_native: HashMap<String, Uint128> = self
            .native
            .iter()
            .map(|x| (x.denom.clone(), x.amount))
            .collect();
        let mut remainders_cw20: HashMap<Addr, Uint128> = self
            .cw20
            .iter()
            .map(|x| (x.address.clone(), x.amount))
            .collect();

        for member_share in distribution {
            let weight_fraction = Decimal::from_ratio(member_share.shares, total_weight);

            let mut split_native = HashMap::new();
            for coin in &self.native {
                let decimal_amount = Decimal::from_atomics(coin.amount, 0u32)?;
                let split_amount = weight_fraction.checked_mul(decimal_amount)?.to_uint_floor();

                // Deduct the split amount from the remainder
                if let Some(remainder) = remainders_native.get_mut(&coin.denom) {
                    *remainder = remainder.checked_sub(split_amount)?;
                }

                split_native.insert(coin.denom.clone(), split_amount);
            }

            let mut split_cw20 = HashMap::new();
            for cw20_coin in &self.cw20 {
                let decimal_amount = Decimal::from_atomics(cw20_coin.amount, 0u32)?;
                let split_amount = weight_fraction.checked_mul(decimal_amount)?.to_uint_floor();

                // Deduct the split amount from the remainder
                if let Some(remainder) = remainders_cw20.get_mut(&cw20_coin.address) {
                    *remainder = remainder.checked_sub(split_amount)?;
                }

                split_cw20.insert(cw20_coin.address.clone(), split_amount);
            }

            let split_balance = Balance {
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

            let member_balance = MemberBalance {
                addr: member_share.addr.clone(),
                balance: split_balance,
            };

            split_balances.push(member_balance);
        }

        // Apply the remainder_balance to the corresponding split_balances entry
        let remainder_balance = Balance {
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

        let remainder_index = split_balances
            .iter()
            .position(|member_balance| member_balance.addr == *remainder_address);

        if let Some(index) = remainder_index {
            let remainder_member_balance = &mut split_balances[index];
            remainder_member_balance.balance = remainder_member_balance
                .balance
                .checked_add(&remainder_balance)?;
        } else {
            let remainder_member_balance = MemberBalance {
                addr: remainder_address.clone(),
                balance: remainder_balance.clone(),
            };
            split_balances.push(remainder_member_balance);
        }
        Ok(split_balances)
    }
}
