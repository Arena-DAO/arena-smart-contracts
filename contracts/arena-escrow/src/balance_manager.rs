use cosmwasm_std::{Addr, Deps, DepsMut, Order, StdResult, Uint128};
use cw_balance::BalanceVerified;
use cw_storage_plus::Map;
use std::collections::{BTreeMap, BTreeSet};

use crate::state::{TOTAL_BALANCE_CW20, TOTAL_BALANCE_CW721, TOTAL_BALANCE_NATIVE};

pub struct BalanceManager<'a> {
    pub native_map: &'a Map<(&'a Addr, &'a str), Uint128>,
    pub cw20_map: &'a Map<(&'a Addr, &'a Addr), Uint128>,
    pub cw721_map: &'a Map<(&'a Addr, &'a Addr, &'a str), ()>,
}

impl<'a> BalanceManager<'a> {
    pub fn new(
        native_map: &'a Map<(&Addr, &str), Uint128>,
        cw20_map: &'a Map<(&Addr, &Addr), Uint128>,
        cw721_map: &'a Map<(&Addr, &Addr, &str), ()>,
    ) -> Self {
        Self {
            native_map,
            cw20_map,
            cw721_map,
        }
    }

    pub fn load_balance(&self, deps: Deps, addr: &Addr) -> StdResult<BalanceVerified> {
        let native = self
            .native_map
            .prefix(addr)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|res| res.map(|(denom, amount)| (denom.to_string(), amount)))
            .collect::<StdResult<_>>()?;

        let cw20 = self
            .cw20_map
            .prefix(addr)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|res| res.map(|(token, amount)| (token.clone(), amount)))
            .collect::<StdResult<_>>()?;

        let mut cw721: BTreeMap<Addr, BTreeSet<String>> = BTreeMap::new();
        for item in
            self.cw721_map
                .sub_prefix(addr)
                .range(deps.storage, None, None, Order::Ascending)
        {
            let ((contract, token_id), _) = item?;
            cw721
                .entry(contract.clone())
                .or_default()
                .insert(token_id.to_string());
        }

        Ok(BalanceVerified {
            native,
            cw20,
            cw721,
        })
    }

    pub fn save_balance(
        &self,
        deps: DepsMut,
        addr: &Addr,
        balance: &BalanceVerified,
    ) -> StdResult<()> {
        for (denom, amount) in &balance.native {
            self.native_map
                .save(deps.storage, (addr, denom.as_str()), amount)?;
        }

        for (token, amount) in &balance.cw20 {
            self.cw20_map.save(deps.storage, (addr, token), amount)?;
        }

        for (contract, token_ids) in &balance.cw721 {
            for token_id in token_ids {
                self.cw721_map
                    .save(deps.storage, (addr, contract, token_id), &())?;
            }
        }

        Ok(())
    }

    pub fn clear_all_balances(&self, deps: DepsMut) -> StdResult<()> {
        self.native_map.clear(deps.storage);
        self.cw20_map.clear(deps.storage);
        self.cw721_map.clear(deps.storage);
        Ok(())
    }

    pub fn clear_user_balance(&self, deps: DepsMut, addr: &Addr) -> StdResult<()> {
        self.native_map.prefix(addr).clear(deps.storage, None);
        self.cw20_map.prefix(addr).clear(deps.storage, None);
        self.cw721_map.sub_prefix(addr).clear(deps.storage, None);
        Ok(())
    }
}

pub struct TotalBalanceManager;

impl TotalBalanceManager {
    pub fn load(deps: Deps) -> StdResult<BalanceVerified> {
        let native = TOTAL_BALANCE_NATIVE
            .range(deps.storage, None, None, Order::Ascending)
            .map(|res| res.map(|(denom, amount)| (denom.to_string(), amount)))
            .collect::<StdResult<_>>()?;

        let cw20 = TOTAL_BALANCE_CW20
            .range(deps.storage, None, None, Order::Ascending)
            .map(|res| res.map(|(addr, amount)| (addr.clone(), amount)))
            .collect::<StdResult<_>>()?;

        let mut cw721: BTreeMap<Addr, BTreeSet<String>> = BTreeMap::new();
        for item in TOTAL_BALANCE_CW721.range(deps.storage, None, None, Order::Ascending) {
            let ((contract, token_id), _) = item?;
            cw721
                .entry(contract.clone())
                .or_default()
                .insert(token_id.to_string());
        }

        Ok(BalanceVerified {
            native,
            cw20,
            cw721,
        })
    }

    pub fn save(deps: DepsMut, balance: &BalanceVerified) -> StdResult<()> {
        for (denom, amount) in &balance.native {
            TOTAL_BALANCE_NATIVE.save(deps.storage, denom.as_str(), amount)?;
        }

        for (token, amount) in &balance.cw20 {
            TOTAL_BALANCE_CW20.save(deps.storage, token, amount)?;
        }

        for (contract, token_ids) in &balance.cw721 {
            for token_id in token_ids {
                TOTAL_BALANCE_CW721.save(deps.storage, (contract, token_id.as_str()), &())?;
            }
        }

        Ok(())
    }

    pub fn clear(deps: DepsMut) -> StdResult<()> {
        TOTAL_BALANCE_NATIVE.clear(deps.storage);
        TOTAL_BALANCE_CW20.clear(deps.storage);
        TOTAL_BALANCE_CW721.clear(deps.storage);
        Ok(())
    }
}
