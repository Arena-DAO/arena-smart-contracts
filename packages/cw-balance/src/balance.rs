use crate::{is_contract, BalanceError, Cw721Collection, Distribution, MemberBalanceChecked};
use cosmwasm_schema::{
    cw_serde,
    schemars::{schema::SchemaObject, JsonSchema},
};
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Coin, CosmosMsg, Decimal, Deps, Empty, StdError, StdResult,
    Uint128, WasmMsg,
};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw721::{
    msg::Cw721ExecuteMsg, DefaultOptionalCollectionExtensionMsg, EmptyOptionalNftExtension,
};
use std::collections::{BTreeMap, BTreeSet};

#[cw_serde]
#[derive(Default)]
pub struct BalanceUnchecked {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub native: Vec<Coin>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cw20: Vec<Cw20Coin>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cw721: Vec<Cw721Collection>,
}

impl BalanceUnchecked {
    pub fn into_checked(self, deps: Deps) -> StdResult<BalanceVerified> {
        Ok(BalanceVerified {
            native: fold_native_coins(self.native)?,
            cw20: fold_cw20_coins(self.cw20, deps)?,
            cw721: fold_cw721_collections(self.cw721, deps)?,
        })
    }
}

pub(crate) fn fold_native_coins(coins: Vec<Coin>) -> StdResult<BTreeMap<String, Uint128>> {
    coins
        .into_iter()
        .try_fold(BTreeMap::new(), |mut map, coin| {
            *map.entry(coin.denom).or_insert(Uint128::zero()) += coin.amount;
            Ok(map)
        })
}

pub(crate) fn fold_cw20_coins(
    coins: Vec<Cw20Coin>,
    deps: Deps,
) -> StdResult<BTreeMap<Addr, Uint128>> {
    coins
        .into_iter()
        .try_fold(BTreeMap::new(), |mut map, coin| {
            let address = deps.api.addr_validate(&coin.address)?;
            *map.entry(address).or_insert(Uint128::zero()) += coin.amount;
            Ok(map)
        })
}

pub(crate) fn fold_cw721_collections(
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

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BalanceVerified {
    pub native: BTreeMap<String, Uint128>,
    pub cw20: BTreeMap<Addr, Uint128>,
    pub cw721: BTreeMap<Addr, BTreeSet<String>>,
}

impl serde::Serialize for BalanceVerified {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("BalanceVerified", 3)?;

        if !self.native.is_empty() {
            state.serialize_field("native", &self.native)?;
        }

        if !self.cw20.is_empty() {
            let cw20_strings: BTreeMap<String, Uint128> = self
                .cw20
                .iter()
                .map(|(addr, amount)| (addr.to_string(), *amount))
                .collect();
            state.serialize_field("cw20", &cw20_strings)?;
        }

        if !self.cw721.is_empty() {
            let cw721_strings: BTreeMap<String, BTreeSet<String>> = self
                .cw721
                .iter()
                .map(|(addr, tokens)| (addr.to_string(), tokens.clone()))
                .collect();
            state.serialize_field("cw721", &cw721_strings)?;
        }

        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for BalanceVerified {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct BalanceVerifiedVisitor;

        impl<'de> Visitor<'de> for BalanceVerifiedVisitor {
            type Value = BalanceVerified;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct BalanceVerified")
            }

            fn visit_map<V>(self, mut map: V) -> Result<BalanceVerified, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut native = None;
                let mut cw20 = None;
                let mut cw721 = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "native" => {
                            if native.is_some() {
                                return Err(de::Error::duplicate_field("native"));
                            }
                            native = Some(map.next_value::<BTreeMap<String, Uint128>>()?);
                        }
                        "cw20" => {
                            if cw20.is_some() {
                                return Err(de::Error::duplicate_field("cw20"));
                            }
                            let cw20_strings: BTreeMap<String, Uint128> = map.next_value()?;
                            let cw20_addrs: BTreeMap<Addr, Uint128> = cw20_strings
                                .into_iter()
                                .map(|(addr_str, amount)| (Addr::unchecked(addr_str), amount))
                                .collect();
                            cw20 = Some(cw20_addrs);
                        }
                        "cw721" => {
                            if cw721.is_some() {
                                return Err(de::Error::duplicate_field("cw721"));
                            }
                            let cw721_strings: BTreeMap<String, BTreeSet<String>> =
                                map.next_value()?;
                            let cw721_addrs: BTreeMap<Addr, BTreeSet<String>> = cw721_strings
                                .into_iter()
                                .map(|(addr_str, tokens)| (Addr::unchecked(addr_str), tokens))
                                .collect();
                            cw721 = Some(cw721_addrs);
                        }
                        _ => {
                            // Ignore unknown fields
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                Ok(BalanceVerified {
                    native: native.unwrap_or_default(),
                    cw20: cw20.unwrap_or_default(),
                    cw721: cw721.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_struct(
            "BalanceVerified",
            &["native", "cw20", "cw721"],
            BalanceVerifiedVisitor,
        )
    }
}

impl JsonSchema for BalanceVerified {
    fn schema_name() -> String {
        "BalanceVerified".to_string()
    }

    fn json_schema(
        gen: &mut cosmwasm_schema::schemars::gen::SchemaGenerator,
    ) -> cosmwasm_schema::schemars::schema::Schema {
        use cosmwasm_schema::schemars::{schema::InstanceType, schema::Schema};
        use std::collections::BTreeMap;

        let mut schema = SchemaObject {
            instance_type: Some(InstanceType::Object.into()),
            ..Default::default()
        };

        let mut properties = BTreeMap::new();
        properties.insert(
            "native".to_string(),
            BTreeMap::<String, Uint128>::json_schema(gen),
        );
        properties.insert(
            "cw20".to_string(),
            BTreeMap::<String, Uint128>::json_schema(gen),
        );
        properties.insert(
            "cw721".to_string(),
            BTreeMap::<String, BTreeSet<String>>::json_schema(gen),
        );

        schema.object().properties = properties;
        Schema::Object(schema)
    }
}

impl BalanceVerified {
    pub fn is_empty(&self) -> bool {
        self.native.is_empty() && self.cw20.is_empty() && self.cw721.is_empty()
    }

    pub fn split(
        balance: &BalanceVerified,
        distribution: &Distribution<Addr>,
        cw721_owners: &BTreeMap<Addr, BTreeMap<Addr, BTreeSet<String>>>,
    ) -> Result<Vec<MemberBalanceChecked>, BalanceError> {
        let mut split_map: BTreeMap<Addr, BalanceVerified> = BTreeMap::new();
        let mut total_split = BalanceVerified::default();

        for member in &distribution.member_percentages {
            let portion = balance.checked_mul_floor(member.percentage)?;
            total_split = total_split.checked_add(&portion)?;
            let entry = split_map.entry(member.addr.clone()).or_default();
            *entry = entry.checked_add(&portion)?;
        }

        let mut remainder = balance.checked_sub(&total_split)?;

        if !cw721_owners.is_empty() {
            let mut routed_nfts: BTreeMap<Addr, BTreeMap<Addr, BTreeSet<String>>> = BTreeMap::new();

            for (contract, token_ids) in &remainder.cw721 {
                for token_id in token_ids {
                    // Find the owner in the provided routing map
                    let mut found = false;
                    for (owner, contracts) in cw721_owners {
                        if let Some(ids) = contracts.get(contract) {
                            if ids.contains(token_id) {
                                routed_nfts
                                    .entry(owner.clone())
                                    .or_default()
                                    .entry(contract.clone())
                                    .or_default()
                                    .insert(token_id.clone());
                                found = true;
                                break;
                            }
                        }
                    }

                    // If not found, assign to remainder_addr
                    if !found {
                        routed_nfts
                            .entry(distribution.remainder_addr.clone())
                            .or_default()
                            .entry(contract.clone())
                            .or_default()
                            .insert(token_id.clone());
                    }
                }
            }

            // Add routed NFTs to the split map
            for (owner, cw721_map) in routed_nfts {
                let entry = split_map.entry(owner).or_default();
                *entry = entry.checked_add(&BalanceVerified {
                    native: BTreeMap::new(),
                    cw20: BTreeMap::new(),
                    cw721: cw721_map,
                })?;
            }

            // Clear NFTs from remainder
            remainder.cw721.clear();
        }

        // Route remaining native/cw20 to remainder_addr
        if !remainder.is_empty() {
            let entry = split_map
                .entry(distribution.remainder_addr.clone())
                .or_default();
            *entry = entry.checked_add(&remainder)?;
        }

        Ok(split_map
            .into_iter()
            .map(|(addr, balance)| MemberBalanceChecked { addr, balance })
            .collect())
    }

    /// Returns a new balance containing only the elements from self that are not fully covered by other.
    /// This is different from checked_sub in that it only reduces the amounts for elements that exist in both,
    /// rather than requiring that all elements in self exist in other.
    pub fn difference_to(&self, other: &BalanceVerified) -> Result<Self, BalanceError> {
        let mut result = self.clone();

        // Handle native tokens
        for (denom, amount) in &other.native {
            if let Some(self_amount) = result.native.get_mut(denom) {
                if *self_amount <= *amount {
                    result.native.remove(denom);
                } else {
                    *self_amount -= *amount;
                }
            }
        }

        // Handle CW20 tokens
        for (token, amount) in &other.cw20 {
            if let Some(self_amount) = result.cw20.get_mut(token) {
                if *self_amount <= *amount {
                    result.cw20.remove(token);
                } else {
                    *self_amount -= *amount;
                }
            }
        }

        // Handle CW721 tokens
        for (contract, token_ids) in &other.cw721 {
            if let Some(self_token_ids) = result.cw721.get_mut(contract) {
                for token_id in token_ids {
                    self_token_ids.remove(token_id);
                }
                if self_token_ids.is_empty() {
                    result.cw721.remove(contract);
                }
            }
        }

        Ok(result)
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

        if !self.native.is_empty() {
            messages.push(CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                to_address: contract_addr.to_string(),
                amount: self
                    .native
                    .iter()
                    .map(|(denom, amount)| Coin {
                        denom: denom.clone(),
                        amount: *amount,
                    })
                    .collect(),
            }));
        }

        for (addr, amount) in &self.cw20 {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: addr.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Send {
                    contract: contract_addr.to_string(),
                    amount: *amount,
                    msg: cw20_msg.clone().unwrap_or_default(),
                })?,
                funds: vec![],
            }));
        }

        for (addr, token_ids) in &self.cw721 {
            for token_id in token_ids {
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: addr.to_string(),
                    msg: to_json_binary(&Cw721ExecuteMsg::<Empty, Empty, Empty>::SendNft {
                        contract: contract_addr.to_string(),
                        token_id: token_id.clone(),
                        msg: cw721_msg.clone().unwrap_or_default(),
                    })?,
                    funds: vec![],
                }));
            }
        }

        Ok(messages)
    }

    fn transfer_all(&self, recipient: &Addr) -> StdResult<Vec<CosmosMsg>> {
        let mut messages = Vec::new();

        if !self.native.is_empty() {
            messages.push(CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                to_address: recipient.to_string(),
                amount: self
                    .native
                    .iter()
                    .map(|(denom, amount)| Coin {
                        denom: denom.clone(),
                        amount: *amount,
                    })
                    .collect(),
            }));
        }

        for (addr, amount) in &self.cw20 {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: addr.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: recipient.to_string(),
                    amount: *amount,
                })?,
                funds: vec![],
            }));
        }

        for (addr, token_ids) in &self.cw721 {
            for token_id in token_ids {
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: addr.to_string(),
                    msg: to_json_binary(&Cw721ExecuteMsg::<
                        EmptyOptionalNftExtension,
                        DefaultOptionalCollectionExtensionMsg,
                        Empty,
                    >::TransferNft {
                        recipient: recipient.to_string(),
                        token_id: token_id.clone(),
                    })?,
                    funds: vec![],
                }));
            }
        }

        Ok(messages)
    }

    pub fn checked_add(&self, other: &BalanceVerified) -> StdResult<Self> {
        let mut native = self.native.clone();
        for (k, v) in &other.native {
            *native.entry(k.clone()).or_insert(Uint128::zero()) += *v;
        }

        let mut cw20 = self.cw20.clone();
        for (k, v) in &other.cw20 {
            *cw20.entry(k.clone()).or_insert(Uint128::zero()) += *v;
        }

        let mut cw721 = self.cw721.clone();
        for (k, v) in &other.cw721 {
            cw721
                .entry(k.clone())
                .or_default()
                .extend(v.iter().cloned());
        }

        Ok(Self {
            native,
            cw20,
            cw721,
        })
    }

    pub fn checked_sub(&self, other: &BalanceVerified) -> Result<Self, BalanceError> {
        let mut native = self.native.clone();
        for (k, v) in &other.native {
            let e = native
                .get_mut(k)
                .ok_or_else(|| StdError::generic_err(format!("Missing native denom: {}", k)))?;
            if *e < *v {
                return Err(StdError::generic_err(format!(
                    "Insufficient native balance for denom: {}",
                    k
                ))
                .into());
            }
            *e -= *v;
            if *e == Uint128::zero() {
                native.remove(k);
            }
        }

        let mut cw20 = self.cw20.clone();
        for (k, v) in &other.cw20 {
            let e = cw20
                .get_mut(k)
                .ok_or_else(|| StdError::generic_err(format!("Missing CW20 token: {}", k)))?;
            if *e < *v {
                return Err(
                    StdError::generic_err(format!("Insufficient CW20 balance for: {}", k)).into(),
                );
            }
            *e -= *v;
            if *e == Uint128::zero() {
                cw20.remove(k);
            }
        }

        let mut cw721 = self.cw721.clone();
        for (k, v) in &other.cw721 {
            let entry = cw721
                .get_mut(k)
                .ok_or_else(|| StdError::generic_err(format!("Missing CW721 collection: {}", k)))?;
            for id in v {
                if !entry.remove(id) {
                    return Err(StdError::generic_err(format!(
                        "Missing CW721 token {} in contract {}",
                        id, k
                    ))
                    .into());
                }
            }
            if entry.is_empty() {
                cw721.remove(k);
            }
        }

        Ok(Self {
            native,
            cw20,
            cw721,
        })
    }

    pub fn checked_mul_floor(&self, multiplier: Decimal) -> Result<Self, BalanceError> {
        if multiplier.is_zero() {
            return Ok(Self::default());
        }
        if multiplier == Decimal::one() {
            return Ok(self.clone());
        }

        let native = self
            .native
            .iter()
            .map(|(k, v)| Ok((k.clone(), v.checked_mul_floor(multiplier)?)))
            .collect::<Result<BTreeMap<_, _>, BalanceError>>()?;

        let cw20 = self
            .cw20
            .iter()
            .map(|(k, v)| Ok((k.clone(), v.checked_mul_floor(multiplier)?)))
            .collect::<Result<BTreeMap<_, _>, BalanceError>>()?;

        let cw721 = if multiplier == Decimal::one() {
            self.cw721.clone()
        } else {
            BTreeMap::new()
        };

        Ok(Self {
            native,
            cw20,
            cw721,
        })
    }
}
