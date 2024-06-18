use arena_tournament_module::state::EliminationType;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, Uint128, Uint64};
use cw_address_like::AddressLike;
use cw_utils::Expiration;

#[cw_serde]
pub struct EnrollmentEntry<T: AddressLike> {
    pub min_members: Option<Uint128>,
    pub max_members: Uint128,
    pub competition_type: CompetitionType,
    pub entry_fee: Option<Coin>,
    pub expiration: Expiration,
    pub category_id: Option<Uint128>,
    pub competition_info: CompetitionInfo<T>,
}

#[cw_serde]
pub enum CompetitionType {
    Wager {},
    League {
        match_win_points: Uint64,
        match_draw_points: Uint64,
        match_lose_points: Uint64,
        distribution: Vec<Decimal>,
    },
    Tournament {
        elimination_type: EliminationType,
        distribution: Vec<Decimal>,
    },
}

#[cw_serde]
pub enum CompetitionInfo<T: AddressLike> {
    Pending {
        host: T,
        name: String,
        description: String,
        expiration: Expiration,
        rules: Vec<String>,
        rulesets: Vec<Uint128>,
        banner: Option<String>,
    },
    Existing(Addr),
}
