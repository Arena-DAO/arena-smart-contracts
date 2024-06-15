use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Decimal, Uint128};
use cw_address_like::AddressLike;
use cw_balance::Distribution;
use cw_utils::Duration;
use dao_interface::state::ModuleInstantiateInfo;
use dao_pre_propose_base::{
    msg::{ExecuteMsg as ExecuteBase, InstantiateMsg as InstantiateBase, QueryMsg as QueryBase},
    state::PreProposeContract,
};
use dao_voting::proposal::SingleChoiceProposeMsg;

use crate::{
    fees::{FeeInformation, TaxConfiguration},
    rating::{MemberResult, Rating},
};

#[cw_serde]
pub struct InstantiateExt {
    pub competition_modules_instantiate_info: Vec<ModuleInstantiateInfo>,
    pub rulesets: Vec<NewRuleset>,
    pub categories: Vec<NewCompetitionCategory>,
    pub tax: Decimal,
    pub tax_configuration: TaxConfiguration,
    pub rating_period: Duration,
}

#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
#[impl_into(ExecuteMsg)]
pub enum ExecuteExt {
    UpdateCompetitionModules {
        to_add: Vec<ModuleInstantiateInfo>,
        to_disable: Vec<String>,
    },
    UpdateTax {
        tax: Decimal,
    },
    UpdateRulesets {
        to_add: Vec<NewRuleset>,
        to_disable: Vec<Uint128>,
    },
    UpdateCategories {
        to_add: Vec<NewCompetitionCategory>,
        to_edit: Vec<EditCompetitionCategory>,
    },
    AdjustRatings {
        category_id: Uint128,
        member_results: Vec<(MemberResult<String>, MemberResult<String>)>,
    },
    UpdateRatingPeriod {
        period: Duration,
    },
}

impl From<ExecuteExt> for ExecuteMsg {
    fn from(msg: ExecuteExt) -> Self {
        ExecuteMsg::Extension { msg }
    }
}

#[cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
#[impl_into(QueryMsg)]
pub enum QueryExt {
    #[returns(Vec<CompetitionModuleResponse<String>>)]
    CompetitionModules {
        start_after: Option<String>,
        limit: Option<u32>,
        include_disabled: Option<bool>,
    },
    #[returns(Ruleset)]
    Ruleset { id: Uint128 },
    #[returns(Vec<Ruleset>)]
    Rulesets {
        category_id: Option<Uint128>,
        start_after: Option<Uint128>,
        limit: Option<u32>,
        include_disabled: Option<bool>,
    },
    #[returns(Decimal)]
    Tax { height: Option<u64> },
    #[returns(CompetitionModuleResponse<String>)]
    CompetitionModule { query: CompetitionModuleQuery },
    #[returns(CompetitionCategory)]
    Category { id: Uint128 },
    #[returns(Vec<CompetitionCategory>)]
    Categories {
        start_after: Option<Uint128>,
        limit: Option<u32>,
        include_disabled: Option<bool>,
    },
    #[returns(bool)]
    IsValidCategoryAndRulesets {
        category_id: Option<Uint128>,
        rulesets: Vec<Uint128>,
    },
    #[returns(DumpStateResponse)]
    DumpState {},
    /// This query is used to get a competition's fee configuration for the Arena tax at its start height
    #[returns(TaxConfigurationResponse)]
    TaxConfig { height: u64 },
    #[returns(Option<Rating>)]
    Rating { category_id: Uint128, addr: String },
    #[returns(Vec<RatingResponse>)]
    RatingLeaderboard {
        category_id: Uint128,
        start_after: Option<(Uint128, String)>,
        limit: Option<u32>,
    },
}

impl From<QueryExt> for QueryMsg {
    fn from(msg: QueryExt) -> Self {
        QueryMsg::QueryExtension { msg }
    }
}

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}

/// This is used to completely generate schema types
/// QueryExt response types are hidden by the QueryBase mapping to Binary output
#[cw_serde]
pub struct SudoMsg {
    pub dump_state_response: DumpStateResponse,
    pub ruleset: Ruleset,
    pub competition_category: CompetitionCategory,
    pub rating: Rating,
}

pub type InstantiateMsg = InstantiateBase<InstantiateExt>;
pub type ExecuteMsg = ExecuteBase<ProposeMessage, ExecuteExt>;
pub type QueryMsg = QueryBase<QueryExt>;
pub type PrePropose = PreProposeContract<InstantiateExt, ExecuteExt, QueryExt, ProposeMessage>;

#[cw_serde]
pub struct DumpStateResponse {
    pub tax: Decimal,
    pub competition_modules: Vec<CompetitionModuleResponse<String>>,
}

#[cw_serde]
pub struct CompetitionModuleResponse<T: AddressLike> {
    pub key: String,
    pub addr: T,
    pub is_enabled: bool,
    pub competition_count: Uint128,
}

#[cw_serde]
pub struct NewRuleset {
    pub category_id: Option<Uint128>,
    pub rules: Vec<String>,
    pub description: String,
}

#[cw_serde]
pub struct NewCompetitionCategory {
    pub name: String,
}

#[cw_serde]
pub enum EditCompetitionCategory {
    Disable { category_id: Uint128 },
    Edit { category_id: Uint128, name: String },
}

#[cw_serde]
pub struct Ruleset {
    pub id: Uint128,
    pub category_id: Option<Uint128>,
    pub rules: Vec<String>,
    pub description: String,
    pub is_enabled: bool,
}

#[cw_serde]
pub struct CompetitionCategory {
    pub id: Uint128,
    pub name: String,
    pub is_enabled: bool,
}

#[cw_serde]
pub struct ProposeMessage {
    pub competition_id: Uint128,
    pub title: String,
    pub description: String,
    pub distribution: Option<Distribution<String>>,
    pub additional_layered_fees: Option<FeeInformation<String>>,
}

#[cw_serde]
pub struct TaxConfigurationResponse {
    pub tax: Decimal,
    pub cw20_msg: Option<Binary>,
    pub cw721_msg: Option<Binary>,
}

#[cw_serde]
pub enum ProposeMessages {
    Propose(SingleChoiceProposeMsg),
}

#[cw_serde]
pub enum CompetitionModuleQuery {
    Key(String, Option<u64>),
    Addr(String),
}

#[cw_serde]
pub struct RatingResponse {
    pub addr: Addr,
    pub rating: Rating,
}
