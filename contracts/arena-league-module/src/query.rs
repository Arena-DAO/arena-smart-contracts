use std::collections::BTreeMap;

use crate::{
    contract::CompetitionModule,
    msg::MemberPoints,
    state::{Match, Result, Round, RoundResponse, MATCHES, ROUNDS},
};
use cosmwasm_std::{Addr, Deps, Order, StdResult, Uint128, Uint64};
use cw_storage_plus::Bound;

pub fn leaderboard(
    deps: Deps,
    league_id: Uint128,
    round: Option<Uint64>,
) -> StdResult<Vec<MemberPoints>> {
    // Load competition details
    let league = CompetitionModule::default()
        .competitions
        .load(deps.storage, league_id.u128())?;

    // Retrieve all rounds for the given league
    let rounds: Vec<Round> = ROUNDS
        .prefix(league_id.u128())
        .range(
            deps.storage,
            None,
            round.map(|x| Bound::inclusive(x.u64())),
            Order::Ascending,
        )
        .map(|x| x.map(|y| y.1))
        .collect::<StdResult<Vec<Round>>>()?;

    // Initialize leaderboard map
    let mut leaderboard: BTreeMap<Addr, (Uint128, Uint64)> = BTreeMap::new();

    // Iterate over each round and each match to populate the leaderboard
    for round in rounds {
        let matches: Vec<Match> = MATCHES
            .prefix((league_id.u128(), round.round_number.u64()))
            .range(deps.storage, None, None, Order::Ascending)
            .map(|x| x.map(|y| y.1))
            .collect::<StdResult<_>>()?;

        for m in matches {
            if let Some(match_result) = m.result {
                let (team_1, team_2) = match match_result {
                    Result::Team1 => (m.team_1, m.team_2),
                    Result::Team2 => (m.team_2, m.team_1),
                    Result::Draw => (m.team_1, m.team_2),
                };

                if match_result != Result::Draw {
                    let points_for_win = league.extension.match_win_points;
                    let points_for_loss = league.extension.match_lose_points;

                    let record_1 = leaderboard
                        .entry(team_1)
                        .or_insert((Uint128::zero(), Uint64::zero()));
                    *record_1 = (
                        record_1.0.checked_add(points_for_win)?,
                        record_1.1.checked_add(Uint64::one())?,
                    );

                    let record_2 = leaderboard
                        .entry(team_2)
                        .or_insert((Uint128::zero(), Uint64::zero()));
                    *record_2 = (
                        record_2.0.checked_add(points_for_loss)?,
                        record_2.1.checked_add(Uint64::one())?,
                    );
                } else {
                    let points_for_draw = league.extension.match_draw_points;

                    let record_1 = leaderboard
                        .entry(team_1)
                        .or_insert((Uint128::zero(), Uint64::zero()));
                    *record_1 = (
                        record_1.0.checked_add(points_for_draw)?,
                        record_1.1.checked_add(Uint64::one())?,
                    );

                    let record_2 = leaderboard
                        .entry(team_2)
                        .or_insert((Uint128::zero(), Uint64::zero()));
                    *record_2 = (
                        record_2.0.checked_add(points_for_draw)?,
                        record_2.1.checked_add(Uint64::one())?,
                    );
                }
            }
        }
    }

    // Create a list of member points
    Ok(leaderboard
        .into_iter()
        .map(|(member, record)| MemberPoints {
            member,
            points: record.0,
            matches_played: record.1,
        })
        .collect())
}

pub fn round(deps: Deps, league_id: Uint128, round_number: Uint64) -> StdResult<RoundResponse> {
    ROUNDS
        .load(deps.storage, (league_id.u128(), round_number.u64()))?
        .into_response(deps, league_id)
}
