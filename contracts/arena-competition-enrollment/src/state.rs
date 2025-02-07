use arena_interface::{
    competition::{
        state::CompetitionResponse,
        types::{CompetitionInfoResponse, CompetitionType, EnrollmentEntryResponse},
    },
    fees::FeeInformation,
    group,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Deps, Empty, StdResult, Timestamp, Uint128, Uint64};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use cw_utils::Expiration;

#[cw_serde]
pub struct LegacyEnrollmentEntry {
    pub min_members: Option<Uint64>,
    pub max_members: Uint64,
    pub entry_fee: Option<Coin>,
    pub expiration: Expiration,
    pub has_finalized: bool,
    pub competition_info: LegacyCompetitionInfo,
    pub competition_type: CompetitionType,
    pub host: Addr,
    pub category_id: Option<Uint128>,
    pub competition_module: Addr,
    pub required_team_size: Option<u32>,
}

#[cw_serde]
pub struct EnrollmentEntry {
    pub min_members: Option<Uint64>,
    pub max_members: Uint64,
    pub entry_fee: Option<Coin>,
    pub duration_before: u64,
    pub has_finalized: bool,
    pub competition_info: CompetitionInfo,
    pub competition_type: CompetitionType,
    pub host: Addr,
    pub category_id: Option<Uint128>,
    pub competition_module: Addr,
    pub required_team_size: Option<u32>,
}

impl EnrollmentEntry {
    pub fn into_response(self, deps: Deps, id: Uint128) -> StdResult<EnrollmentEntryResponse> {
        let competition_info = self
            .competition_info
            .into_response(deps, &self.competition_module)?;
        let current_members: Uint64 = deps.querier.query_wasm_smart(
            competition_info.group_contract.to_string(),
            &group::QueryMsg::MembersCount {},
        )?;

        Ok(EnrollmentEntryResponse {
            category_id: self.category_id,
            id,
            current_members,
            min_members: self.min_members,
            max_members: self.max_members,
            entry_fee: self.entry_fee,
            duration_before: self.duration_before,
            has_finalized: self.has_finalized,
            competition_info,
            competition_type: self.competition_type,
            host: self.host,
            competition_module: self.competition_module,
            required_team_size: self.required_team_size,
        })
    }
}

#[cw_serde]
pub enum LegacyCompetitionInfo {
    Pending {
        name: String,
        description: String,
        expiration: Expiration,
        rules: Option<Vec<String>>,
        rulesets: Option<Vec<Uint128>>,
        banner: Option<String>,
        additional_layered_fees: Option<Vec<FeeInformation<Addr>>>,
        escrow: Addr,
        group_contract: Addr,
    },
    Existing {
        id: Uint128,
    },
}

#[cw_serde]
pub enum CompetitionInfo {
    Pending {
        name: String,
        description: String,
        date: Timestamp,
        duration: u64,
        rules: Option<Vec<String>>,
        rulesets: Option<Vec<Uint128>>,
        banner: Option<String>,
        additional_layered_fees: Option<Vec<FeeInformation<Addr>>>,
        escrow: Addr,
        group_contract: Addr,
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
                date,
                duration,
                rules,
                rulesets,
                banner,
                additional_layered_fees,
                group_contract,
                escrow,
            } => CompetitionInfoResponse {
                name,
                description,
                date,
                duration,
                rules,
                rulesets,
                banner,
                additional_layered_fees,
                competition_id: None,
                escrow,
                group_contract,
            },
            CompetitionInfo::Existing { id } => {
                let competition = deps
                    .querier
                    .query_wasm_smart::<CompetitionResponse<Empty>>(
                        module_addr.to_string(),
                        &arena_interface::competition::msg::QueryBase::Competition::<
                            Empty,
                            Empty,
                            Empty,
                        > {
                            competition_id: id,
                        },
                    )?;

                CompetitionInfoResponse {
                    name: competition.name,
                    description: competition.description,
                    rules: competition.rules,
                    rulesets: competition.rulesets,
                    banner: competition.banner,
                    duration: competition.duration,
                    date: competition.date,
                    additional_layered_fees: competition.fees,
                    competition_id: Some(id),
                    escrow: competition.escrow,
                    group_contract: competition.group_contract,
                }
            }
        })
    }
}

pub struct EnrollmentEntryIndexes<'a> {
    pub category: MultiIndex<'a, u128, EnrollmentEntry, u128>,
    pub host: MultiIndex<'a, String, EnrollmentEntry, u128>,
}

impl IndexList<EnrollmentEntry> for EnrollmentEntryIndexes<'_> {
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
    pub escrow_addr: Addr,
}

/// MIGRATIONS
pub const LEGACY_ENROLLMENTS: Map<u128, LegacyEnrollmentEntry> = Map::new("enrollment_entries");
