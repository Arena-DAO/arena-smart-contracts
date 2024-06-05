use cosmwasm_std::{Deps, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

use crate::state::{Match, MATCHES};

pub fn query_bracket(
    deps: Deps,
    tournament_id: Uint128,
    start_after: Option<Uint128>,
) -> StdResult<Vec<Match>> {
    let start_after_bound = start_after.map(Bound::exclusive);

    MATCHES
        .prefix(tournament_id.u128())
        .range(deps.storage, start_after_bound, None, Order::Ascending)
        .map(|x| x.map(|y| y.1))
        .collect::<StdResult<Vec<_>>>()
}
