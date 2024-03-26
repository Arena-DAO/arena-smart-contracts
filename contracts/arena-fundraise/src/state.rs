use std::fmt::Display;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Uint128};
use cw_storage_plus::{Item, Map};
use cw_utils::Expiration;

#[cw_serde]
pub struct Config {
    pub state: FundraiseState,
    pub fundraise: Coin,
    pub deposit_denom: String,
    pub soft_cap: Uint128,
    pub hard_cap: Option<Uint128>,
    pub start: Option<Expiration>,
    pub end: Expiration,
    pub recipient: Addr,
}

#[cw_serde]
pub enum FundraiseState {
    Active,
    Failed,
    Successful,
}

impl Display for FundraiseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FundraiseState::Active => write!(f, "Active"),
            FundraiseState::Failed => write!(f, "Failed"),
            FundraiseState::Successful => write!(f, "Successful"),
        }
    }
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const TOTAL_DEPOSITED: Item<Uint128> = Item::new("total_deposited");
pub const USER_DEPOSIT: Map<&Addr, Uint128> = Map::new("user_deposit");
