use std::{cmp::Reverse, collections::BTreeMap};

use crate::{
    contract::CompetitionModule,
    msg::{DumpStateResponse, MemberPoints, PointAdjustmentResponse, RoundResponse},
    state::{Match, MatchResult, MATCHES, POINT_ADJUSTMENTS, ROUNDS},
};
use cosmwasm_std::{Addr, Deps, Int128, Order, StdResult, Uint128, Uint64};
use cw_storage_plus::Bound;

/// Calculates and returns the leaderboard for a specific league.
///
/// # Arguments
/// * `deps` - Dependencies providing access to storage
/// * `league_id` - Unique identifier of the league
/// * `round_number` - Optional round number to limit the calculation up to
///
/// # Returns
/// Returns a `StdResult` containing a `Vec<MemberPoints>`. On success, the vector contains
/// `MemberPoints` structs for each member in the league, sorted in descending order by points.
///
/// # Details
/// - The function calculates points based on match results and any point adjustments.
/// - It processes all rounds up to `round_number` if specified, or all rounds if None.
/// - Match results are processed in descending order within each round.
/// - Point adjustments are applied after processing all matches.
/// - The final leaderboard is sorted by points (highest to lowest).
/// - In case of ties (equal points), no additional tie-breaking mechanism is applied.
///
/// # Errors
/// This function will return an error if:
/// - The specified league cannot be found
/// - There's an error reading from storage
/// - There's an arithmetic overflow when calculating points
///
/// # Performance
/// - Time complexity: O(m log m + n log n), where m is the number of matches and n is the number of members
/// - Space complexity: O(n) for storing the leaderboard
///
/// # Note
/// Tie breaking is handled in conjunction with this query
pub fn leaderboard(
    deps: Deps,
    league_id: Uint128,
    round_number: Option<Uint64>,
) -> StdResult<Vec<MemberPoints>> {
    // Load competition details
    let league = CompetitionModule::default()
        .competitions
        .load(deps.storage, league_id.u128())?;

    // Initialize leaderboard map
    let mut leaderboard: BTreeMap<Addr, (Int128, Uint64)> = BTreeMap::new();

    // Determine the range of rounds to process
    let end_bound = round_number.map(|x| Bound::inclusive(x.u64()));

    // Process rounds and matches in a single pass
    for round in
        ROUNDS
            .prefix(league_id.u128())
            .range(deps.storage, None, end_bound, Order::Ascending)
    {
        let (_, round) = round?;

        for match_key in MATCHES
            .prefix((league_id.u128(), round.round_number.u64()))
            .keys(deps.storage, None, None, Order::Descending)
        {
            let match_key = match_key?;
            let m: Match = MATCHES.load(
                deps.storage,
                (league_id.u128(), round.round_number.u64(), match_key),
            )?;

            if let Some(match_result) = m.result {
                let (winner, loser) = match match_result {
                    MatchResult::Team1 => (m.team_1, m.team_2),
                    MatchResult::Team2 => (m.team_2, m.team_1),
                    MatchResult::Draw => {
                        update_leaderboard(
                            &mut leaderboard,
                            m.team_1,
                            league.extension.match_draw_points.into(),
                        )?;
                        update_leaderboard(
                            &mut leaderboard,
                            m.team_2,
                            league.extension.match_draw_points.into(),
                        )?;
                        continue;
                    }
                };

                update_leaderboard(
                    &mut leaderboard,
                    winner,
                    league.extension.match_win_points.into(),
                )?;
                update_leaderboard(
                    &mut leaderboard,
                    loser,
                    league.extension.match_lose_points.into(),
                )?;
            }
        }
    }

    // Apply point adjustments in a single pass
    for point_adjustment in
        POINT_ADJUSTMENTS
            .prefix(league_id.u128())
            .range(deps.storage, None, None, Order::Ascending)
    {
        let (addr, adjustments) = point_adjustment?;
        if let Some(record) = leaderboard.get_mut(&addr) {
            record.0 += adjustments.iter().map(|x| x.amount).sum::<Int128>();
        }
    }

    // Convert to Vec, sort by points (descending), and map to MemberPoints
    let mut sorted_leaderboard: Vec<MemberPoints> = leaderboard
        .into_iter()
        .map(|(member, (points, matches_played))| MemberPoints {
            member,
            points,
            matches_played,
        })
        .collect();

    sorted_leaderboard.sort_unstable_by_key(|mp| Reverse(mp.points));

    Ok(sorted_leaderboard)
}

fn update_leaderboard(
    leaderboard: &mut BTreeMap<Addr, (Int128, Uint64)>,
    team: Addr,
    points: Int128,
) -> StdResult<()> {
    let record = leaderboard
        .entry(team)
        .or_insert((Int128::zero(), Uint64::zero()));
    record.0 = record.0.checked_add(points)?;
    record.1 = record.1.checked_add(Uint64::one())?;
    Ok(())
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
    let limit = limit.unwrap_or(30).max(30);

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
