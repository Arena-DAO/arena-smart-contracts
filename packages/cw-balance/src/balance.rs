use crate::{
    cw721::Cw721CollectionVerified, is_contract, BalanceError, Cw721Collection, Distribution,
    MemberBalanceChecked,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Coin, CosmosMsg, Decimal, Deps, StdError, StdResult, Uint128,
    WasmMsg,
};
use cw20::{Cw20Coin, Cw20CoinVerified, Cw20ExecuteMsg};
use cw721::Cw721ExecuteMsg;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

#[cw_serde]
pub struct BalanceUnchecked {
    pub native: Option<Vec<Coin>>,
    pub cw20: Option<Vec<Cw20Coin>>,
    pub cw721: Option<Vec<Cw721Collection>>,
}

impl BalanceUnchecked {
    pub fn into_checked(self, deps: Deps) -> StdResult<BalanceVerified> {
        let native = self.native.map(fold_native_coins).transpose()?;
        let cw20 = self
            .cw20
            .map(|coins| fold_cw20_coins(coins, deps))
            .transpose()?;
        let cw721 = self
            .cw721
            .map(|collections| fold_cw721_collections(collections, deps))
            .transpose()?;

        Ok(BalanceVerified::from_maps(native, cw20, cw721))
    }
}

fn fold_native_coins(coins: Vec<Coin>) -> StdResult<BTreeMap<String, Uint128>> {
    coins
        .into_iter()
        .try_fold(BTreeMap::new(), |mut map, coin| {
            *map.entry(coin.denom).or_insert(Uint128::zero()) += coin.amount;
            Ok(map)
        })
}

fn fold_cw20_coins(coins: Vec<Cw20Coin>, deps: Deps) -> StdResult<BTreeMap<Addr, Uint128>> {
    coins
        .into_iter()
        .try_fold(BTreeMap::new(), |mut map, coin| {
            let address = deps.api.addr_validate(&coin.address)?;
            *map.entry(address).or_insert(Uint128::zero()) += coin.amount;
            Ok(map)
        })
}

fn fold_cw721_collections(
    collections: Vec<Cw721Collection>,
    deps: Deps,
) -> StdResult<BTreeMap<Addr, BTreeSet<String>>> {
    collections.into_iter().try_fold(
        BTreeMap::new(),
        |mut map: BTreeMap<Addr, BTreeSet<String>>, collection| {
            let address = deps.api.addr_validate(&collection.address)?;
            let token_ids: BTreeSet<String> = collection.token_ids.iter().cloned().collect();
            if token_ids.len() != collection.token_ids.len() {
                return Err(StdError::generic_err(format!(
                    "Duplicate CW721 token IDs for contract {}",
                    address
                )));
            }
            map.entry(address).or_default().extend(token_ids);
            Ok(map)
        },
    )
}

#[cw_serde]
#[derive(Default)]
pub struct BalanceVerified {
    pub native: Option<Vec<Coin>>,
    pub cw20: Option<Vec<Cw20CoinVerified>>,
    pub cw721: Option<Vec<Cw721CollectionVerified>>,
}

impl fmt::Display for BalanceVerified {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let native_str = self.format_native();
        let cw20_str = self.format_cw20();
        let cw721_str = self.format_cw721();

        write!(
            f,
            "Native: [{}], CW20: [{}], CW721: [{}]",
            native_str, cw20_str, cw721_str
        )
    }
}

impl BalanceVerified {
    pub fn new() -> Self {
        Self::default()
    }

    fn format_native(&self) -> String {
        match &self.native {
            Some(coins) => coins
                .iter()
                .map(|coin| format!("{}: {}", coin.denom, coin.amount))
                .collect::<Vec<_>>()
                .join(", "),
            None => "None".to_string(),
        }
    }

    fn format_cw20(&self) -> String {
        match &self.cw20 {
            Some(tokens) => tokens
                .iter()
                .map(|token| format!("{}: {}", token.address, token.amount))
                .collect::<Vec<_>>()
                .join(", "),
            None => "None".to_string(),
        }
    }

    fn format_cw721(&self) -> String {
        match &self.cw721 {
            Some(collections) => collections
                .iter()
                .map(|collection| {
                    let tokens = collection.token_ids.join(", ");
                    format!("{}: [{}]", collection.address, tokens)
                })
                .collect::<Vec<_>>()
                .join("; "),
            None => "None".to_string(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.native.as_ref().map_or(true, Vec::is_empty)
            && self.cw20.as_ref().map_or(true, Vec::is_empty)
            && self.cw721.as_ref().map_or(true, Vec::is_empty)
    }

    #[allow(clippy::type_complexity)]
    fn to_maps(
        &self,
    ) -> (
        Option<BTreeMap<String, Uint128>>,
        Option<BTreeMap<Addr, Uint128>>,
        Option<BTreeMap<Addr, BTreeSet<String>>>,
    ) {
        (
            self.native
                .as_ref()
                .map(|v| v.iter().map(|c| (c.denom.clone(), c.amount)).collect()),
            self.cw20
                .as_ref()
                .map(|v| v.iter().map(|c| (c.address.clone(), c.amount)).collect()),
            self.cw721.as_ref().map(|v| {
                v.iter()
                    .map(|c| (c.address.clone(), c.token_ids.iter().cloned().collect()))
                    .collect()
            }),
        )
    }

    #[allow(clippy::type_complexity)]
    fn from_maps(
        native: Option<BTreeMap<String, Uint128>>,
        cw20: Option<BTreeMap<Addr, Uint128>>,
        cw721: Option<BTreeMap<Addr, BTreeSet<String>>>,
    ) -> Self {
        BalanceVerified {
            native: native.and_then(|m| {
                if m.is_empty() {
                    None
                } else {
                    Some(
                        m.into_iter()
                            .map(|(denom, amount)| Coin { denom, amount })
                            .collect(),
                    )
                }
            }),
            cw20: cw20.and_then(|m| {
                if m.is_empty() {
                    None
                } else {
                    Some(
                        m.into_iter()
                            .map(|(address, amount)| Cw20CoinVerified { address, amount })
                            .collect(),
                    )
                }
            }),
            cw721: cw721.and_then(|m| {
                if m.is_empty() {
                    None
                } else {
                    Some(
                        m.into_iter()
                            .map(|(address, token_ids)| Cw721CollectionVerified {
                                address,
                                token_ids: token_ids.into_iter().collect(),
                            })
                            .collect(),
                    )
                }
            }),
        }
    }

    pub fn checked_add(&self, other: &BalanceVerified) -> StdResult<Self> {
        let (self_native, self_cw20, self_cw721) = self.to_maps();
        let (other_native, other_cw20, other_cw721) = other.to_maps();

        let native = merge_maps(&self_native, &other_native, |a, b| Ok(a.checked_add(*b)?));
        let cw20 = merge_maps(&self_cw20, &other_cw20, |a, b| Ok(a.checked_add(*b)?));
        let cw721 = merge_cw721_maps(&self_cw721, &other_cw721);

        Ok(Self::from_maps(native?, cw20?, cw721?))
    }

    pub fn checked_sub(&self, other: &BalanceVerified) -> Result<Self, BalanceError> {
        let (self_native, self_cw20, self_cw721) = self.to_maps();
        let (other_native, other_cw20, other_cw721) = other.to_maps();

        let native = subtract_maps(&self_native, &other_native, "native")?;
        let cw20 = subtract_maps(&self_cw20, &other_cw20, "cw20")?;
        let cw721 = subtract_cw721_maps(&self_cw721, &other_cw721)?;

        Ok(Self::from_maps(native, cw20, cw721))
    }

    pub fn checked_mul_floor(&self, multiplier: Decimal) -> Result<Self, BalanceError> {
        if multiplier.is_zero() {
            return Ok(Self::default());
        }

        let (native, cw20, cw721) = self.to_maps();

        let native = native
            .map(|m| {
                m.into_iter()
                    .map(|(k, v)| Ok((k, v.checked_mul_floor(multiplier)?)))
                    .collect::<Result<BTreeMap<_, _>, BalanceError>>()
            })
            .transpose()?;

        let cw20 = cw20
            .map(|m| {
                m.into_iter()
                    .map(|(k, v)| Ok((k, v.checked_mul_floor(multiplier)?)))
                    .collect::<Result<BTreeMap<_, _>, BalanceError>>()
            })
            .transpose()?;

        let cw721 = if multiplier == Decimal::one() {
            cw721
        } else {
            None
        };

        Ok(Self::from_maps(native, cw20, cw721))
    }

    pub fn split(
        &self,
        distribution: &Distribution<Addr>,
    ) -> Result<Vec<MemberBalanceChecked>, BalanceError> {
        let mut split_balances = Vec::with_capacity(distribution.member_percentages.len());
        let (native, cw20, cw721) = self.to_maps();

        // Calculate split balances
        for member_percentage in &distribution.member_percentages {
            let native_split = native.as_ref().map(|m| {
                m.iter()
                    .map(|(k, &v)| {
                        (
                            k.clone(),
                            v.checked_mul_floor(member_percentage.percentage)
                                .unwrap_or_default(),
                        )
                    })
                    .filter(|(_, v)| !v.is_zero())
                    .collect::<BTreeMap<_, _>>()
            });

            let cw20_split = cw20.as_ref().map(|m| {
                m.iter()
                    .map(|(k, &v)| {
                        (
                            k.clone(),
                            v.checked_mul_floor(member_percentage.percentage)
                                .unwrap_or_default(),
                        )
                    })
                    .filter(|(_, v)| !v.is_zero())
                    .collect::<BTreeMap<_, _>>()
            });

            let cw721_split = if member_percentage.percentage == Decimal::one() {
                cw721.clone()
            } else {
                None
            };

            let split_balance = BalanceVerified::from_maps(native_split, cw20_split, cw721_split);
            split_balances.push(MemberBalanceChecked {
                addr: member_percentage.addr.clone(),
                balance: split_balance,
            });
        }

        // Calculate remainders
        let total_split = split_balances
            .iter()
            .try_fold(BalanceVerified::new(), |acc, mb| {
                acc.checked_add(&mb.balance)
            })?;
        let remainders = self.checked_sub(&total_split)?;

        // Distribute remainders
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

    fn send_all(
        &self,
        contract_addr: &Addr,
        cw20_msg: Option<Binary>,
        cw721_msg: Option<Binary>,
    ) -> StdResult<Vec<CosmosMsg>> {
        let mut messages = Vec::new();

        // Send native tokens
        messages.extend(self.send_native(contract_addr));

        // Send CW20 tokens
        if let Some(cw20) = &self.cw20 {
            for cw20_coin in cw20 {
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cw20_coin.address.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Send {
                        contract: contract_addr.to_string(),
                        amount: cw20_coin.amount,
                        msg: cw20_msg.clone().unwrap_or_default(),
                    })?,
                    funds: vec![],
                }));
            }
        }

        // Send CW721 tokens
        if let Some(cw721) = &self.cw721 {
            for collection in cw721 {
                for token_id in &collection.token_ids {
                    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: collection.address.to_string(),
                        msg: to_json_binary(&Cw721ExecuteMsg::SendNft {
                            contract: contract_addr.to_string(),
                            token_id: token_id.clone(),
                            msg: cw721_msg.clone().unwrap_or_default(),
                        })?,
                        funds: vec![],
                    }));
                }
            }
        }

        Ok(messages)
    }

    fn transfer_all(&self, recipient: &Addr) -> StdResult<Vec<CosmosMsg>> {
        let mut messages = Vec::new();

        // Transfer native tokens
        messages.extend(self.send_native(recipient));

        // Transfer CW20 tokens
        if let Some(cw20) = &self.cw20 {
            for cw20_coin in cw20 {
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cw20_coin.address.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: recipient.to_string(),
                        amount: cw20_coin.amount,
                    })?,
                    funds: vec![],
                }));
            }
        }

        // Transfer CW721 tokens
        if let Some(cw721) = &self.cw721 {
            for collection in cw721 {
                for token_id in &collection.token_ids {
                    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: collection.address.to_string(),
                        msg: to_json_binary(&Cw721ExecuteMsg::TransferNft {
                            recipient: recipient.to_string(),
                            token_id: token_id.clone(),
                        })?,
                        funds: vec![],
                    }));
                }
            }
        }

        Ok(messages)
    }

    fn send_native(&self, address: &Addr) -> Vec<CosmosMsg> {
        if let Some(native) = &self.native {
            if !native.is_empty() {
                vec![CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                    to_address: address.to_string(),
                    amount: native.clone(),
                })]
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    pub fn difference_to(&self, other: &BalanceVerified) -> Result<Self, BalanceError> {
        let (self_native, self_cw20, self_cw721) = self.to_maps();
        let (other_native, other_cw20, other_cw721) = other.to_maps();

        let native_diff = calculate_diff(&other_native, &self_native)?;
        let cw20_diff = calculate_diff(&other_cw20, &self_cw20)?;
        let cw721_diff = calculate_cw721_diff(&other_cw721, &self_cw721)?;

        Ok(Self::from_maps(native_diff, cw20_diff, cw721_diff))
    }
}

fn merge_maps<K, V, F>(
    a: &Option<BTreeMap<K, V>>,
    b: &Option<BTreeMap<K, V>>,
    merge_fn: F,
) -> StdResult<Option<BTreeMap<K, V>>>
where
    K: Ord + Clone,
    V: Clone,
    F: Fn(&V, &V) -> StdResult<V>,
{
    match (a, b) {
        (Some(a), Some(b)) if !a.is_empty() || !b.is_empty() => {
            let mut result = a.clone();
            for (k, v) in b {
                if let Some(e) = result.get_mut(k) {
                    *e = merge_fn(e, v)?;
                } else {
                    result.insert(k.clone(), v.clone());
                }
            }
            Ok(Some(result))
        }
        (Some(a), None) if !a.is_empty() => Ok(Some(a.clone())),
        (None, Some(b)) if !b.is_empty() => Ok(Some(b.clone())),
        _ => Ok(None),
    }
}

fn merge_cw721_maps(
    a: &Option<BTreeMap<Addr, BTreeSet<String>>>,
    b: &Option<BTreeMap<Addr, BTreeSet<String>>>,
) -> StdResult<Option<BTreeMap<Addr, BTreeSet<String>>>> {
    merge_maps(a, b, |a, b| {
        let mut combined = a.clone();
        combined.extend(b.iter().cloned());
        Ok(combined)
    })
}

fn subtract_maps<K, V>(
    a: &Option<BTreeMap<K, V>>,
    b: &Option<BTreeMap<K, V>>,
    token_type: &str,
) -> StdResult<Option<BTreeMap<K, V>>>
where
    K: Ord + Clone + std::fmt::Display,
    V: Copy + PartialOrd + std::ops::Sub<Output = V> + Default,
{
    match (a, b) {
        (Some(a), Some(b)) if !a.is_empty() => {
            let mut result = a.clone();
            for (k, v) in b {
                if let Some(existing) = result.get_mut(k) {
                    if *v > *existing {
                        return Err(StdError::generic_err(format!(
                            "Insufficient {} balance for {}",
                            token_type, k
                        )));
                    }
                    *existing = *existing - *v;
                    if *existing == V::default() {
                        result.remove(k);
                    }
                } else {
                    return Err(StdError::generic_err(format!(
                        "Insufficient {} balance for {}",
                        token_type, k
                    )));
                }
            }
            Ok(if result.is_empty() {
                None
            } else {
                Some(result)
            })
        }
        (Some(a), None) if !a.is_empty() => Ok(Some(a.clone())),
        (None, Some(b)) if !b.is_empty() => Err(StdError::generic_err(format!(
            "Insufficient {} balance",
            token_type
        ))),
        _ => Ok(None),
    }
}

fn subtract_cw721_maps(
    a: &Option<BTreeMap<Addr, BTreeSet<String>>>,
    b: &Option<BTreeMap<Addr, BTreeSet<String>>>,
) -> StdResult<Option<BTreeMap<Addr, BTreeSet<String>>>> {
    match (a, b) {
        (Some(a), Some(b)) if !a.is_empty() => {
            let mut result = a.clone();
            for (addr, token_ids) in b {
                if let Some(existing) = result.get_mut(addr) {
                    for token_id in token_ids {
                        if !existing.remove(token_id) {
                            return Err(StdError::generic_err(format!(
                                "CW721 token {} not found for contract {}",
                                token_id, addr
                            )));
                        }
                    }
                    if existing.is_empty() {
                        result.remove(addr);
                    }
                } else {
                    return Err(StdError::generic_err(format!(
                        "No CW721 tokens found for contract {}",
                        addr
                    )));
                }
            }
            Ok(if result.is_empty() {
                None
            } else {
                Some(result)
            })
        }
        (Some(a), None) if !a.is_empty() => Ok(Some(a.clone())),
        (None, Some(b)) if !b.is_empty() => {
            Err(StdError::generic_err("Insufficient CW721 balance"))
        }
        _ => Ok(None),
    }
}

fn calculate_diff<K>(
    target: &Option<BTreeMap<K, Uint128>>,
    current: &Option<BTreeMap<K, Uint128>>,
) -> StdResult<Option<BTreeMap<K, Uint128>>>
where
    K: Ord + Clone + std::fmt::Display,
{
    match (target, current) {
        (Some(target), Some(current)) if !target.is_empty() => {
            let mut result = BTreeMap::new();
            for (k, &v) in target {
                let diff = match current.get(k) {
                    Some(&current_value) if v > current_value => v.checked_sub(current_value)?,
                    None => v,
                    _ => Uint128::zero(),
                };
                if !diff.is_zero() {
                    result.insert(k.clone(), diff);
                }
            }
            Ok(if result.is_empty() {
                None
            } else {
                Some(result)
            })
        }
        (Some(target), None) if !target.is_empty() => Ok(Some(target.clone())),
        _ => Ok(None),
    }
}

fn calculate_cw721_diff(
    target: &Option<BTreeMap<Addr, BTreeSet<String>>>,
    current: &Option<BTreeMap<Addr, BTreeSet<String>>>,
) -> StdResult<Option<BTreeMap<Addr, BTreeSet<String>>>> {
    match (target, current) {
        (Some(target), Some(current)) if !target.is_empty() => {
            let mut result = BTreeMap::new();
            for (addr, token_ids) in target {
                let diff = match current.get(addr) {
                    Some(current_tokens) => token_ids.difference(current_tokens).cloned().collect(),
                    None => token_ids.clone(),
                };
                if !diff.is_empty() {
                    result.insert(addr.clone(), diff);
                }
            }
            Ok(if result.is_empty() {
                None
            } else {
                Some(result)
            })
        }
        (Some(target), None) if !target.is_empty() => Ok(Some(target.clone())),
        _ => Ok(None),
    }
}
