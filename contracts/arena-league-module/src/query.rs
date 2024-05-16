use std::collections::BTreeMap;

use crate::{
    contract::CompetitionModule,
    msg::{DumpStateResponse, MemberPoints, PointAdjustmentResponse, RoundResponse},
    state::{Match, Result, Round, MATCHES, POINT_ADJUSTMENTS, ROUNDS},
};
use cosmwasm_std::{Addr, Deps, Int128, Order, StdResult, Uint128, Uint64};
use cw_storage_plus::Bound;

pub fn leaderboard(
    deps: Deps,
    league_id: Uint128,
    round_number: Option<Uint64>,
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
            round_number.map(|x| Bound::inclusive(x.u64())),
            Order::Ascending,
        )
        .map(|x| x.map(|y| y.1))
        .collect::<StdResult<Vec<Round>>>()?;

    // Initialize leaderboard map
    let mut leaderboard: BTreeMap<Addr, (Int128, Uint64)> = BTreeMap::new();

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
                        .or_insert((Int128::zero(), Uint64::zero()));
                    *record_1 = (
                        record_1.0.checked_add(points_for_win.into())?,
                        record_1.1.checked_add(Uint64::one())?,
                    );

                    let record_2 = leaderboard
                        .entry(team_2)
                        .or_insert((Int128::zero(), Uint64::zero()));
                    *record_2 = (
                        record_2.0.checked_add(points_for_loss.into())?,
                        record_2.1.checked_add(Uint64::one())?,
                    );
                } else {
                    let points_for_draw = league.extension.match_draw_points;

                    let record_1 = leaderboard
                        .entry(team_1)
                        .or_insert((Int128::zero(), Uint64::zero()));
                    *record_1 = (
                        record_1.0.checked_add(points_for_draw.into())?,
                        record_1.1.checked_add(Uint64::one())?,
                    );

                    let record_2 = leaderboard
                        .entry(team_2)
                        .or_insert((Int128::zero(), Uint64::zero()));
                    *record_2 = (
                        record_2.0.checked_add(points_for_draw.into())?,
                        record_2.1.checked_add(Uint64::one())?,
                    );
                }
            }
        }
    }

    // Apply point adjustments
    if !POINT_ADJUSTMENTS.is_empty(deps.storage) {
        for (addr, (points, _matches_played)) in leaderboard.iter_mut() {
            if POINT_ADJUSTMENTS.has(deps.storage, (league_id.u128(), addr)) {
                let point_adjustments =
                    POINT_ADJUSTMENTS.load(deps.storage, (league_id.u128(), addr))?;

                *points += point_adjustments.iter().map(|x| x.amount).sum::<Int128>();
            }
        }
    }

    // Create a list of member points
    Ok(leaderboard
        .into_iter()
        .map(|(member, (points, matches_played))| MemberPoints {
            member,
            points,
            matches_played,
        })
        .collect())
}

pub fn round(deps: Deps, league_id: Uint128, round_number: Uint64) -> StdResult<RoundResponse> {
    ROUNDS
        .load(deps.storage, (league_id.u128(), round_number.u64()))?
        .into_response(deps, league_id)
}

pub fn point_adjustments(
    deps: Deps,
    league_id: Uint128,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<PointAdjustmentResponse>> {
    let start_after = start_after
        .map(|x| deps.api.addr_validate(&x))
        .transpose()?;
    let start_after = start_after.as_ref().map(Bound::exclusive);
    let limit = limit.unwrap_or(10).max(30);

    POINT_ADJUSTMENTS
        .prefix(league_id.u128())
        .range(deps.storage, start_after, None, Order::Descending)
        .map(|x| {
            x.map(|(addr, point_adjustments)| PointAdjustmentResponse {
                addr,
                point_adjustments,
            })
        })
        .take(limit as usize)
        .collect::<StdResult<Vec<_>>>()
}

pub fn dump_state(
    deps: Deps,
    league_id: Uint128,
    round_number: Uint64,
) -> StdResult<DumpStateResponse> {
    Ok(DumpStateResponse {
        leaderboard: leaderboard(deps, league_id, None)?,
        round: round(deps, league_id, round_number)?,
        point_adjustments: point_adjustments(deps, league_id, None, None)?,
    })
}
