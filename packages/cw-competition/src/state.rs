use arena_core_interface::msg::Ruleset;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, Deps, StdResult, Uint128};
use cw_balance::MemberShare;
use cw_ownable::get_ownership;
use cw_utils::Expiration;
use std::fmt;

#[cw_serde]
#[derive(Default)]
pub enum CompetitionStatus {
    Pending,
    Active,
    #[default]
    Inactive,
    Jailed,
}

impl fmt::Display for CompetitionStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompetitionStatus::Pending => write!(f, "Pending"),
            CompetitionStatus::Jailed => write!(f, "Jailed"),
            CompetitionStatus::Active => write!(f, "Active"),
            CompetitionStatus::Inactive => write!(f, "Inactive"),
        }
    }
}

#[cw_serde]
pub struct Competition<CompetitionExt> {
    pub id: Uint128,
    pub admin_dao: Addr,
    pub dao: Addr,
    pub escrow: Option<Addr>,
    pub name: String,
    pub description: String,
    pub start_height: u64,
    pub expiration: Expiration,
    pub rules: Vec<String>,
    pub ruleset: Option<Uint128>,
    pub status: CompetitionStatus,
    pub extension: CompetitionExt,
    pub has_generated_proposals: bool,
    pub result: Option<Vec<MemberShare<Addr>>>,
}

/// CompetitionResponse has all of the same fields as Competition
/// is_expired is appended
#[cw_serde]
pub struct CompetitionResponse<CompetitionExt> {
    pub id: Uint128,
    pub dao: Addr,
    pub escrow: Option<Addr>,
    pub name: String,
    pub description: String,
    pub start_height: u64,
    pub is_expired: bool,
    pub rules: Vec<String>,
    pub ruleset: Option<Ruleset>,
    pub status: CompetitionStatus,
    pub extension: CompetitionExt,
    pub has_generated_proposals: bool,
    pub expiration: Expiration,
    pub result: Option<Vec<MemberShare<Addr>>>,
}

impl<CompetitionExt> Competition<CompetitionExt> {
    pub fn to_response(
        self,
        deps: Deps,
        block: &BlockInfo,
        include_ruleset: Option<bool>,
    ) -> StdResult<CompetitionResponse<CompetitionExt>> {
        let is_expired = self.expiration.is_expired(block);

        let mut ruleset: Option<Ruleset> = None;

        if let Some(ruleset_id) = self.ruleset {
            if include_ruleset.unwrap_or(true) {
                let owner = get_ownership(deps.storage)?;

                if let Some(owner) = owner.owner {
                    ruleset = deps.querier.query_wasm_smart(
                        owner.to_string(),
                        &arena_core_interface::msg::QueryMsg::QueryExtension {
                            msg: arena_core_interface::msg::QueryExt::Ruleset { id: ruleset_id },
                        },
                    )?;
                }
            }
        }

        Ok(CompetitionResponse {
            id: self.id,
            dao: self.dao,
            escrow: self.escrow,
            name: self.name,
            description: self.description,
            start_height: self.start_height,
            is_expired,
            rules: self.rules,
            ruleset,
            status: self.status,
            extension: self.extension,
            has_generated_proposals: self.has_generated_proposals,
            expiration: self.expiration,
            result: self.result,
        })
    }
}

#[cw_serde]
pub struct Config {
    pub key: String,
    pub description: String,
}
