use std::collections::BTreeMap;

use crate::{
    contract::CompetitionModule,
    msg::MemberPoints,
    state::{rounds, Match, Round},
};
use cosmwasm_std::{Addr, Deps, StdResult, Uint128, Uint64};

pub fn leaderboard(deps: Deps, league_id: Uint128) -> StdResult<Vec<MemberPoints>> {
    let league = CompetitionModule::default()
        .competitions
        .load(deps.storage, league_id.u128())?;

    let rounds: Vec<Round> = rounds()
        .prefix(league_id.u128())
        .range(deps.storage, None, None, cosmwasm_std::Order::Descending)
        .map(|x| x.map(|y| y.1))
        .collect::<StdResult<Vec<Round>>>()?;

    let matches: Vec<Match> = rounds
        .into_iter()
        .flat_map(|round| round.matches.into_iter())
        .filter(|m| m.result.is_some())
        .collect();

    let mut leaderboard: BTreeMap<Addr, (Uint128, Uint64)> = BTreeMap::new();
    for m in matches {
        if let Some(match_result) = m.result {
            match match_result.result {
                Some(winner) => {
                    let (team_1, team_2) = match winner {
                        true => (m.team_1, m.team_2),
                        false => (m.team_2, m.team_1),
                    };

                    let record_1 = leaderboard.entry(team_1).or_default();
                    *record_1 = (
                        record_1.0.checked_add(league.extension.match_win_points)?,
                        record_1.1.checked_add(Uint64::one())?,
                    );

                    let record_2 = leaderboard.entry(team_2).or_default();
                    *record_2 = (
                        record_2.0.checked_add(league.extension.match_lose_points)?,
                        record_2.1.checked_add(Uint64::one())?,
                    );
                }
                None => {
                    let record_1 = leaderboard.entry(m.team_1).or_default();
                    *record_1 = (
                        record_1.0.checked_add(league.extension.match_draw_points)?,
                        record_1.1.checked_add(Uint64::one())?,
                    );

                    let record_2 = leaderboard.entry(m.team_2).or_default();
                    *record_2 = (
                        record_2.0.checked_add(league.extension.match_draw_points)?,
                        record_2.1.checked_add(Uint64::one())?,
                    );
                }
            }
        }
    }

    Ok(leaderboard
        .into_iter()
        .map(|(member, record)| MemberPoints {
            member,
            points: record.0,
            matches_played: record.1,
        })
        .collect())
}

pub fn round(deps: Deps, league_id: Uint128, round_number: Uint64) -> StdResult<Round> {
    rounds().load(deps.storage, (league_id.u128(), round_number.u64()))
}
