use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Deps, Int128, StdResult, Uint128, Uint64};
use cw_storage_plus::Map;

use crate::msg::RoundResponse;

#[cw_serde]
pub struct TournamentExt {
    pub tax_cw20_msg: Option<Binary>,
    pub tax_cw721_msg: Option<Binary>,
}

#[cw_serde]
pub struct Match {
    pub match_number: Uint128,
    pub team_1: Addr,
    pub team_2: Addr,
    pub result: Option<Result>,
}

#[cw_serde]
pub enum Result {
    Team1,
    Team2,
    Draw,
}

#[cw_serde]
pub struct Round {
    pub round_number: Uint64,
    pub matches: Vec<Uint128>, // A link to the Match by match_number
}

impl Round {
    pub fn into_response(self, deps: Deps, league_id: Uint128) -> StdResult<RoundResponse> {
        let matches = MATCHES
            .prefix((league_id.u128(), self.round_number.u64()))
            .range(deps.storage, None, None, cosmwasm_std::Order::Descending)
            .map(|x| x.map(|y| y.1))
            .collect::<StdResult<Vec<Match>>>()?;

        Ok(RoundResponse {
            round_number: self.round_number,
            matches,
        })
    }
}

#[cw_serde]
pub struct PointAdjustment {
    pub description: String,
    pub amount: Int128,
}

/// (League Id, Round Number)
pub const ROUNDS: Map<(u128, u64), Round> = Map::new("rounds");
/// (League Id, Round Number, Match Number)
pub const MATCHES: Map<(u128, u64, u128), Match> = Map::new("matches");
/// (League Id, Addr)
pub const POINT_ADJUSTMENTS: Map<(u128, &Addr), Vec<PointAdjustment>> =
    Map::new("point_adjustments");
