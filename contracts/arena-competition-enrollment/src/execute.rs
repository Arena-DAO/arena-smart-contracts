use cosmwasm_std::{ensure, Coin, DepsMut, Env, MessageInfo, Response, StdError, Uint128};
use cw_utils::Expiration;

use crate::{state::CompetitionInfo, ContractError};

pub fn create_competition(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    min_members: Option<Uint128>,
    max_members: Uint128,
    entry_fee: Option<Coin>,
    expiration: Expiration,
    category_id: Option<Uint128>,
    competition_info: CompetitionInfo<String>,
) -> Result<Response, ContractError> {
    ensure!(
        !matches!(competition_info, CompetitionInfo::Existing(_)),
        ContractError::StdError(StdError::generic_err("Competition info cannot be existing"))
    );
    todo!()
}
