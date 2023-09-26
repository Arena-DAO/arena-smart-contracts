use std::marker::PhantomData;

use crate::state::CompetitionStatus;
#[allow(unused_imports)]
use crate::state::{CompetitionResponse, Config};
use arena_core_interface::msg::ProposeMessage;
use cosmwasm_schema::{cw_serde, schemars::JsonSchema, QueryResponses};
use cosmwasm_std::Uint128;
use cw_balance::MemberShare;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use cw_utils::Expiration;
use dao_interface::state::ModuleInstantiateInfo;

#[cw_serde]
pub struct InstantiateBase<InstantiateExt> {
    pub key: String, //this is used to map a key (wager, tournament, league) to a module
    pub description: String,
    pub extension: InstantiateExt,
}

#[cw_ownable_execute]
#[cw_serde]
#[allow(clippy::large_enum_variant)]
pub enum ExecuteBase<ExecuteExt, CompetitionExt> {
    JailCompetition {
        propose_message: ProposeMessage,
    },
    Activate {},
    DeclareResult {
        propose_message: ProposeMessage,
    },
    CreateCompetition {
        competition_dao: ModuleInstantiateInfo,
        escrow: Option<ModuleInstantiateInfo>,
        name: String,
        description: String,
        expiration: Expiration,
        rules: Vec<String>,
        ruleset: Option<Uint128>,
        extension: CompetitionExt,
    },
    ProcessCompetition {
        id: Uint128,
        distribution: Option<Vec<MemberShare<String>>>,
    },
    Extension {
        msg: ExecuteExt,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryBase<QueryExt, CompetitionExt>
where
    QueryExt: JsonSchema,
{
    #[returns(Config)]
    Config {},
    #[returns(Uint128)]
    CompetitionCount {},
    #[returns(CompetitionResponse<CompetitionExt>)]
    Competition {
        id: Uint128,
        include_ruleset: Option<bool>, // Defaults to true
    },
    #[returns(Vec<CompetitionResponse<CompetitionExt>>)]
    Competitions {
        start_after: Option<Uint128>,
        limit: Option<u32>,
        include_ruleset: Option<bool>,
        status: Option<CompetitionStatus>,
    },
    #[returns(cosmwasm_std::Binary)]
    QueryExtension { msg: QueryExt },
    #[serde(skip)]
    #[returns(PhantomData<CompetitionExt>)]
    _Phantom(PhantomData<CompetitionExt>),
}
