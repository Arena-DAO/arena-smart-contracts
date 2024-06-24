use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, StdError, StdResult,
    Uint128, WasmMsg,
};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw721::Cw721ExecuteMsg;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::ops::AddAssign;

use crate::{is_contract, BalanceError, Cw721Collection, Distribution};

#[cw_serde]
pub struct MemberBalanceChecked {
    pub addr: Addr,
    pub balance: BalanceVerified,
}

#[cw_serde]
pub struct MemberBalanceUnchecked {
    pub addr: String,
    pub balance: BalanceUnchecked,
}

impl MemberBalanceUnchecked {
    pub fn into_checked(self, deps: Deps) -> StdResult<MemberBalanceChecked> {
        Ok(MemberBalanceChecked {
            addr: deps.api.addr_validate(&self.addr)?,
            balance: self.balance.into_checked(deps)?,
        })
    }
}

#[cw_serde]
pub struct BalanceUnchecked {
    pub native: Option<Vec<Coin>>,
    pub cw20: Option<Vec<Cw20Coin>>,
    pub cw721: Option<Vec<Cw721Collection>>,
}

impl BalanceUnchecked {
    pub fn into_checked(self, deps: Deps) -> StdResult<BalanceVerified> {
        let native = self.native.map(|coins| {
            coins.into_iter().fold(BTreeMap::new(), |mut map, coin| {
                map.entry(coin.denom)
                    .or_insert(Uint128::zero())
                    .add_assign(coin.amount);
                map
            })
        });

        let cw20 = self
            .cw20
            .map(|coins| {
                coins
                    .into_iter()
                    .try_fold(BTreeMap::new(), |mut map, coin| -> StdResult<_> {
                        let address = deps.api.addr_validate(&coin.address)?;
                        map.entry(address)
                            .or_insert(Uint128::zero())
                            .add_assign(coin.amount);
                        Ok(map)
                    })
            })
            .transpose()?;

        let cw721 = self
            .cw721
            .map(|collections| {
                collections.into_iter().try_fold(
                    BTreeMap::new(),
                    |mut map, collection| -> StdResult<_> {
                        let address = deps.api.addr_validate(&collection.address)?;
                        let original_len = collection.token_ids.len();
                        let token_ids: BTreeSet<String> =
                            collection.token_ids.into_iter().collect();
                        if token_ids.len() != original_len {
                            return Err(StdError::generic_err("Duplicate CW721 token IDs"));
                        }
                        map.insert(address, token_ids);
                        Ok(map)
                    },
                )
            })
            .transpose()?;

        Ok(BalanceVerified {
            native,
            cw20,
            cw721,
        })
    }
}

#[derive(Default)]
#[cw_serde]
pub struct BalanceVerified {
    pub native: Option<BTreeMap<String, Uint128>>,
    pub cw20: Option<BTreeMap<Addr, Uint128>>,
    pub cw721: Option<BTreeMap<Addr, BTreeSet<String>>>,
}

impl Display for BalanceVerified {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut has_entries = false;

        if let Some(native) = &self.native {
            if !native.is_empty() {
                has_entries = true;
                writeln!(f, "Native Balances:")?;
                for (denom, amount) in native {
                    writeln!(f, "  {} {}", amount, denom)?;
                }
            }
        }

        if let Some(cw20) = &self.cw20 {
            if !cw20.is_empty() {
                if has_entries {
                    writeln!(f)?;
                }
                has_entries = true;
                writeln!(f, "CW20 Balances:")?;
                for (addr, amount) in cw20 {
                    writeln!(f, "  {} {}", amount, addr)?;
                }
            }
        }

        if let Some(cw721) = &self.cw721 {
            if !cw721.is_empty() {
                if has_entries {
                    writeln!(f)?;
                }
                writeln!(f, "CW721 Balances:")?;
                for (addr, token_ids) in cw721 {
                    writeln!(f, "  {} ({} tokens)", addr, token_ids.len())?;
                }
            }
        }

        if !has_entries {
            writeln!(f, "No balances available")?;
        }

        Ok(())
    }
}

impl BalanceVerified {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.native.as_ref().map_or(true, BTreeMap::is_empty)
            && self.cw20.as_ref().map_or(true, BTreeMap::is_empty)
            && self.cw721.as_ref().map_or(true, BTreeMap::is_empty)
    }

    pub fn checked_add(&self, other: &BalanceVerified) -> Result<Self, BalanceError> {
        Ok(Self {
            native: merge_btreemaps(&self.native, &other.native, |a, b| Ok(a.checked_add(*b)?))?,
            cw20: merge_btreemaps(&self.cw20, &other.cw20, |a, b| Ok(a.checked_add(*b)?))?,
            cw721: merge_btreemaps(&self.cw721, &other.cw721, |a, b| {
                let mut combined = a.clone();
                combined.extend(b.iter().cloned());
                Ok(combined)
            })?,
        })
    }

    pub fn checked_sub(&self, other: &BalanceVerified) -> Result<Self, BalanceError> {
        Ok(Self {
            native: subtract_btreemaps(&self.native, &other.native)?,
            cw20: subtract_btreemaps(&self.cw20, &other.cw20)?,
            cw721: subtract_cw721_btreemaps(&self.cw721, &other.cw721)?,
        })
    }

    pub fn checked_mul_floor(&self, percentage: Decimal) -> Result<Self, BalanceError> {
        Ok(Self {
            native: self
                .native
                .as_ref()
                .map(|native| {
                    native
                        .iter()
                        .filter_map(|(denom, amount)| {
                            amount
                                .checked_mul_floor(percentage)
                                .map(|new_amount| {
                                    (!new_amount.is_zero()).then(|| (denom.clone(), new_amount))
                                })
                                .transpose()
                        })
                        .collect::<Result<BTreeMap<_, _>, _>>()
                })
                .transpose()?
                .filter(|m| !m.is_empty()),
            cw20: self
                .cw20
                .as_ref()
                .map(|cw20| {
                    cw20.iter()
                        .filter_map(|(addr, amount)| {
                            amount
                                .checked_mul_floor(percentage)
                                .map(|new_amount| {
                                    (!new_amount.is_zero()).then(|| (addr.clone(), new_amount))
                                })
                                .transpose()
                        })
                        .collect::<Result<BTreeMap<_, _>, _>>()
                })
                .transpose()?
                .filter(|m| !m.is_empty()),
            cw721: None, // CW721 tokens are not split
        })
    }

    pub fn difference(&self, other: &BalanceVerified) -> StdResult<BalanceVerified> {
        let native = self.difference_map(&self.native, &other.native)?;
        let cw20 = self.difference_map(&self.cw20, &other.cw20)?;
        let cw721 = self.difference_cw721(&self.cw721, &other.cw721)?;

        Ok(BalanceVerified {
            native,
            cw20,
            cw721,
        })
    }

    fn difference_map<K, V>(
        &self,
        a: &Option<BTreeMap<K, V>>,
        b: &Option<BTreeMap<K, V>>,
    ) -> StdResult<Option<BTreeMap<K, V>>>
    where
        K: Ord + Clone,
        V: Copy + PartialOrd + std::ops::Sub<Output = V> + Default,
    {
        match (a, b) {
            (Some(a), Some(b)) => {
                let mut diff = BTreeMap::new();
                for (key, &b_value) in b.iter() {
                    if let Some(&a_value) = a.get(key) {
                        if b_value > a_value {
                            diff.insert(key.clone(), b_value - a_value);
                        }
                    } else {
                        diff.insert(key.clone(), b_value);
                    }
                }
                if diff.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(diff))
                }
            }
            (None, Some(b)) => Ok(Some(b.clone())),
            _ => Ok(None),
        }
    }

    fn difference_cw721(
        &self,
        a: &Option<BTreeMap<Addr, BTreeSet<String>>>,
        b: &Option<BTreeMap<Addr, BTreeSet<String>>>,
    ) -> StdResult<Option<BTreeMap<Addr, BTreeSet<String>>>> {
        match (a, b) {
            (Some(a), Some(b)) => {
                let mut diff = BTreeMap::new();
                for (addr, b_tokens) in b.iter() {
                    if let Some(a_tokens) = a.get(addr) {
                        let token_diff: BTreeSet<_> =
                            b_tokens.difference(a_tokens).cloned().collect();
                        if !token_diff.is_empty() {
                            diff.insert(addr.clone(), token_diff);
                        }
                    } else {
                        diff.insert(addr.clone(), b_tokens.clone());
                    }
                }
                if diff.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(diff))
                }
            }
            (None, Some(b)) => Ok(Some(b.clone())),
            _ => Ok(None),
        }
    }

    pub fn transmit_all(
        &self,
        deps: Deps,
        recipient: &Addr,
        cw20_msg: Option<Binary>,
        cw721_msg: Option<Binary>,
    ) -> StdResult<Vec<CosmosMsg>> {
        if is_contract(deps, recipient.to_string()) {
            self.send_all(recipient, cw20_msg, cw721_msg)
        } else {
            self.transfer_all(recipient)
        }
    }

    pub fn transfer_all(&self, recipient: &Addr) -> StdResult<Vec<CosmosMsg>> {
        let mut messages = Vec::new();

        messages.extend(self.send_native(recipient));
        messages.extend(self.transfer_cw20(recipient)?);
        messages.extend(self.transfer_cw721(recipient)?);

        Ok(messages)
    }

    pub fn send_all(
        &self,
        contract_addr: &Addr,
        cw20_msg: Option<Binary>,
        cw721_msg: Option<Binary>,
    ) -> StdResult<Vec<CosmosMsg>> {
        let mut messages = Vec::new();

        messages.extend(self.send_native(contract_addr));
        messages.extend(self.send_cw20(contract_addr, cw20_msg.unwrap_or_default())?);
        messages.extend(self.send_cw721(contract_addr, cw721_msg.unwrap_or_default())?);

        Ok(messages)
    }

    pub fn send_native(&self, address: &Addr) -> Vec<CosmosMsg> {
        if let Some(native) = &self.native {
            if !native.is_empty() {
                vec![CosmosMsg::Bank(BankMsg::Send {
                    to_address: address.to_string(),
                    amount: native
                        .iter()
                        .map(|(denom, amount)| Coin {
                            denom: denom.clone(),
                            amount: *amount,
                        })
                        .collect(),
                })]
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    pub fn send_cw20(&self, contract: &Addr, msg: Binary) -> StdResult<Vec<CosmosMsg>> {
        if let Some(cw20) = &self.cw20 {
            cw20.iter()
                .map(|(token_addr, amount)| {
                    let exec_msg = WasmMsg::Execute {
                        contract_addr: token_addr.to_string(),
                        msg: to_json_binary(&Cw20ExecuteMsg::Send {
                            contract: contract.to_string(),
                            amount: *amount,
                            msg: msg.clone(),
                        })?,
                        funds: vec![],
                    };
                    Ok(CosmosMsg::Wasm(exec_msg))
                })
                .collect()
        } else {
            Ok(vec![])
        }
    }

    pub fn send_cw721(&self, contract: &Addr, msg: Binary) -> StdResult<Vec<CosmosMsg>> {
        if let Some(cw721) = &self.cw721 {
            cw721
                .iter()
                .flat_map(|(nft_addr, token_ids)| {
                    token_ids.iter().map(|token_id| {
                        let exec_msg = WasmMsg::Execute {
                            contract_addr: nft_addr.to_string(),
                            msg: to_json_binary(&Cw721ExecuteMsg::SendNft {
                                contract: contract.to_string(),
                                token_id: token_id.clone(),
                                msg: msg.clone(),
                            })?,
                            funds: vec![],
                        };
                        Ok(CosmosMsg::Wasm(exec_msg))
                    })
                })
                .collect()
        } else {
            Ok(vec![])
        }
    }

    pub fn transfer_cw721(&self, recipient: &Addr) -> StdResult<Vec<CosmosMsg>> {
        if let Some(cw721) = &self.cw721 {
            cw721
                .iter()
                .flat_map(|(nft_addr, token_ids)| {
                    token_ids.iter().map(|token_id| {
                        let exec_msg = WasmMsg::Execute {
                            contract_addr: nft_addr.to_string(),
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
        } else {
            Ok(vec![])
        }
    }

    pub fn transfer_cw20(&self, recipient: &Addr) -> StdResult<Vec<CosmosMsg>> {
        if let Some(cw20) = &self.cw20 {
            cw20.iter()
                .map(|(token_addr, amount)| {
                    let exec_msg = WasmMsg::Execute {
                        contract_addr: token_addr.to_string(),
                        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                            recipient: recipient.to_string(),
                            amount: *amount,
                        })?,
                        funds: vec![],
                    };
                    Ok(CosmosMsg::Wasm(exec_msg))
                })
                .collect()
        } else {
            Ok(vec![])
        }
    }

    pub fn split(
        &self,
        distribution: &Distribution<Addr>,
    ) -> Result<Vec<MemberBalanceChecked>, BalanceError> {
        let mut split_balances: Vec<MemberBalanceChecked> = Vec::new();

        let mut remainders = self.clone();

        for member_percentage in &distribution.member_percentages {
            let split_balance = self.checked_mul_floor(member_percentage.percentage)?;
            remainders = remainders.checked_sub(&split_balance)?;

            split_balances.push(MemberBalanceChecked {
                addr: member_percentage.addr.clone(),
                balance: split_balance,
            });
        }

        if !remainders.is_empty() {
            if let Some(member_balance) = split_balances
                .iter_mut()
                .find(|mb| mb.addr == distribution.remainder_addr)
            {
                member_balance.balance = member_balance.balance.checked_add(&remainders)?;
            } else {
                split_balances.push(MemberBalanceChecked {
                    addr: distribution.remainder_addr.clone(),
                    balance: remainders,
                });
            }
        }

        Ok(split_balances)
    }
}

fn merge_btreemaps<K, V, F>(
    a: &Option<BTreeMap<K, V>>,
    b: &Option<BTreeMap<K, V>>,
    merge_fn: F,
) -> Result<Option<BTreeMap<K, V>>, BalanceError>
where
    K: Ord + Clone,
    V: Clone,
    F: Fn(&V, &V) -> Result<V, BalanceError>,
{
    match (a, b) {
        (Some(a), Some(b)) => {
            let mut result = a.clone();
            for (k, v) in b {
                match result.entry(k.clone()) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(v.clone());
                    }
                    std::collections::btree_map::Entry::Occupied(mut entry) => {
                        let merged_value = merge_fn(entry.get(), v)?;
                        entry.insert(merged_value);
                    }
                }
            }
            Ok(Some(result))
        }
        (Some(a), None) => Ok(Some(a.clone())),
        (None, Some(b)) => Ok(Some(b.clone())),
        (None, None) => Ok(None),
    }
}

fn subtract_btreemaps<K, V>(
    a: &Option<BTreeMap<K, V>>,
    b: &Option<BTreeMap<K, V>>,
) -> Result<Option<BTreeMap<K, V>>, BalanceError>
where
    K: Ord + Clone,
    V: Copy + PartialOrd + std::ops::Sub<Output = V> + Default,
{
    match (a, b) {
        (Some(a), Some(b)) => {
            let mut result = a.clone();
            for (k, v) in b {
                if let Some(existing) = result.get_mut(k) {
                    if *v > *existing {
                        return Err(BalanceError::InsufficientBalance);
                    }
                    *existing = *existing - *v;
                    if *existing == V::default() {
                        result.remove(k);
                    }
                } else {
                    return Err(BalanceError::InsufficientBalance);
                }
            }
            if result.is_empty() {
                Ok(None)
            } else {
                Ok(Some(result))
            }
        }
        (Some(a), None) => Ok(Some(a.clone())),
        (None, Some(_)) => Err(BalanceError::InsufficientBalance),
        (None, None) => Ok(None),
    }
}

fn subtract_cw721_btreemaps(
    a: &Option<BTreeMap<Addr, BTreeSet<String>>>,
    b: &Option<BTreeMap<Addr, BTreeSet<String>>>,
) -> Result<Option<BTreeMap<Addr, BTreeSet<String>>>, BalanceError> {
    match (a, b) {
        (Some(a), Some(b)) => {
            let mut result = a.clone();
            for (addr, token_ids) in b {
                if let Some(existing) = result.get_mut(addr) {
                    if !token_ids.is_subset(existing) {
                        return Err(BalanceError::InsufficientBalance);
                    }
                    *existing = existing.difference(token_ids).cloned().collect();
                    if existing.is_empty() {
                        result.remove(addr);
                    }
                } else {
                    return Err(BalanceError::InsufficientBalance);
                }
            }
            if result.is_empty() {
                Ok(None)
            } else {
                Ok(Some(result))
            }
        }
        (Some(a), None) => Ok(Some(a.clone())),
        (None, Some(_)) => Err(BalanceError::InsufficientBalance),
        (None, None) => Ok(None),
    }
}
