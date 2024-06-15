use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Empty, StdError, StdResult};
use cw_competition::{
    msg::{ExecuteBase, InstantiateBase, QueryBase, ToCompetitionExt},
    state::{Competition, CompetitionResponse},
};

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}

#[cw_serde]
pub struct WagerInstantiateExt {
    // This should be set if the match should update ratings
    pub registered_members: Option<Vec<String>>,
}

#[cw_serde]
pub struct WagerExt {
    pub registered_members: Option<Vec<Addr>>,
}

pub type InstantiateMsg = InstantiateBase<Empty>;
pub type ExecuteMsg = ExecuteBase<Empty, WagerInstantiateExt>;
pub type QueryMsg = QueryBase<Empty, Empty, WagerExt>;
pub type Wager = Competition<WagerExt>;
pub type WagerResponse = CompetitionResponse<WagerExt>;

impl ToCompetitionExt<WagerExt> for WagerInstantiateExt {
    fn to_competition_ext(&self, deps: cosmwasm_std::Deps) -> cosmwasm_std::StdResult<WagerExt> {
        if let Some(registered_members) = &self.registered_members {
            if registered_members.len() != 2 {
                return Err(StdError::generic_err(
                    "Registered members must be of length 2",
                ));
            }

            return Ok(WagerExt {
                registered_members: Some(
                    registered_members
                        .iter()
                        .map(|x| deps.api.addr_validate(x))
                        .collect::<StdResult<_>>()?,
                ),
            });
        }

        Ok(WagerExt {
            registered_members: None,
        })
    }
}
