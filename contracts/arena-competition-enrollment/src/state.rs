use std::fmt;

use arena_interface::{competition::state::CompetitionResponse, fees::FeeInformation, group};
use arena_tournament_module::state::EliminationType;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, Coin, Decimal, Deps, Empty, StdResult, Uint128, Uint64};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};
use cw_utils::Expiration;

#[cw_serde]
pub struct EnrollmentEntry {
    pub min_members: Option<Uint64>,
    pub max_members: Uint64,
    pub entry_fee: Option<Coin>,
    pub expiration: Expiration,
    pub has_triggered_expiration: bool,
    pub competition_info: CompetitionInfo,
    pub competition_type: CompetitionType,
    pub host: Addr,
    pub category_id: Option<Uint128>,
    pub competition_module: Addr,
    pub group_contract: Addr,
}

#[cw_serde]
pub struct EnrollmentEntryResponse {
    pub category_id: Option<Uint128>,
    pub id: Uint128,
    pub current_members: Uint64,
    pub min_members: Option<Uint64>,
    pub max_members: Uint64,
    pub entry_fee: Option<Coin>,
    pub expiration: Expiration,
    pub has_triggered_expiration: bool,
    pub competition_info: CompetitionInfoResponse,
    pub competition_type: CompetitionType,
    pub host: Addr,
    pub is_expired: bool,
    pub competition_module: Addr,
    pub group_contract: Addr,
}

#[cw_serde]
pub struct CompetitionInfoResponse {
    name: String,
    description: String,
    expiration: Expiration,
    rules: Option<Vec<String>>,
    rulesets: Option<Vec<Uint128>>,
    banner: Option<String>,
    additional_layered_fees: Option<Vec<FeeInformation<String>>>,
    competition_id: Option<Uint128>,
}

impl EnrollmentEntry {
    pub fn into_response(
        self,
        deps: Deps,
        block: &BlockInfo,
        id: Uint128,
    ) -> StdResult<EnrollmentEntryResponse> {
        let current_members: Uint64 = deps.querier.query_wasm_smart(
            self.group_contract.to_string(),
            &group::QueryMsg::MembersCount {},
        )?;
        let is_expired = self.expiration.is_expired(block);

        Ok(EnrollmentEntryResponse {
            category_id: self.category_id,
            id,
            current_members,
            min_members: self.min_members,
            max_members: self.max_members,
            entry_fee: self.entry_fee,
            expiration: self.expiration,
            has_triggered_expiration: self.has_triggered_expiration,
            competition_info: self
                .competition_info
                .into_response(deps, &self.competition_module)?,
            competition_type: self.competition_type,
            host: self.host,
            is_expired,
            competition_module: self.competition_module,
            group_contract: self.group_contract,
        })
    }
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

impl fmt::Display for CompetitionType {
    /// This value should match up the module key
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompetitionType::Wager {} => write!(f, "Wagers"),
            CompetitionType::League { .. } => write!(f, "Leagues"),
            CompetitionType::Tournament { .. } => write!(f, "Tournaments"),
        }
    }
}

#[cw_serde]
pub enum CompetitionInfo {
    Pending {
        name: String,
        description: String,
        expiration: Expiration,
        rules: Option<Vec<String>>,
        rulesets: Option<Vec<Uint128>>,
        banner: Option<String>,
        additional_layered_fees: Option<Vec<FeeInformation<String>>>,
    },
    Existing {
        id: Uint128,
    },
}

impl CompetitionInfo {
    pub fn into_response(
        self,
        deps: Deps,
        module_addr: &Addr,
    ) -> StdResult<CompetitionInfoResponse> {
        Ok(match self {
            CompetitionInfo::Pending {
                name,
                description,
                expiration,
                rules,
                rulesets,
                banner,
                additional_layered_fees,
            } => CompetitionInfoResponse {
                name,
                description,
                expiration,
                rules,
                rulesets,
                banner,
                additional_layered_fees,
                competition_id: None,
            },
            CompetitionInfo::Existing { id } => {
                let competition = deps
                    .querier
                    .query_wasm_smart::<CompetitionResponse<Empty>>(
                        module_addr.to_string(),
                        &arena_interface::competition::msg::QueryBase::<Empty, Empty, Empty>::Competition {
                            competition_id: id,
                        },
                    )?;

                CompetitionInfoResponse {
                    name: competition.name,
                    description: competition.description,
                    rules: competition.rules,
                    rulesets: competition.rulesets,
                    banner: competition.banner,
                    expiration: competition.expiration,
                    additional_layered_fees: None, // We don't need to know this information here, because it will be on the escrow
                    competition_id: Some(id),
                }
            }
        })
    }
}

pub struct EnrollmentEntryIndexes<'a> {
    pub category: MultiIndex<'a, u128, EnrollmentEntry, u128>,
    pub host: MultiIndex<'a, String, EnrollmentEntry, u128>,
}

impl<'a> IndexList<EnrollmentEntry> for EnrollmentEntryIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<EnrollmentEntry>> + '_> {
        let v: Vec<&dyn Index<EnrollmentEntry>> = vec![&self.host, &self.category];
        Box::new(v.into_iter())
    }
}

pub fn enrollment_entries<'a>() -> IndexedMap<'a, u128, EnrollmentEntry, EnrollmentEntryIndexes<'a>>
{
    let indexes = EnrollmentEntryIndexes {
        category: MultiIndex::new(
            |_x, d: &EnrollmentEntry| d.category_id.unwrap_or(Uint128::zero()).u128(),
            "enrollment_entries",
            "enrollment_entries__category",
        ),
        host: MultiIndex::new(
            |_x, d: &EnrollmentEntry| d.host.to_string(),
            "enrollment_entries",
            "enrollment_entries__host",
        ),
    };
    IndexedMap::new("enrollment_entries", indexes)
}

pub const ENROLLMENT_COUNT: Item<Uint128> = Item::new("enrollment_count");
/// Stores the module address and enrollment id to process in a reply
pub const TEMP_ENROLLMENT_INFO: Item<EnrollmentInfo> = Item::new("temp_enrollment_info");

#[cw_serde]
pub struct EnrollmentInfo {
    pub module_addr: Addr,
    pub enrollment_id: u128,
    pub amount: Option<Coin>,
}
