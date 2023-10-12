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
pub enum ExecuteBase<ExecuteExt, CompetitionExt, CompetitionInstantiateExt> {
    JailCompetition {
        propose_message: ProposeMessage,
    },
    Activate {},
    ProposeResult {
        propose_message: ProposeMessage,
    },
    AddCompetitionHook {
        id: Uint128,
    },
    RemoveCompetitionHook {
        id: Uint128,
    },
    ExecuteCompetitionHook {
        id: Uint128,
        distribution: Vec<MemberShare<String>>,
    },
    CreateCompetition {
        competition_dao: ModuleInstantiateInfo,
        escrow: Option<ModuleInstantiateInfo>,
        name: String,
        description: String,
        expiration: Expiration,
        rules: Vec<String>,
        rulesets: Vec<Uint128>,
        extension: CompetitionExt,
        instantiate_extension: CompetitionInstantiateExt,
    },
    ProcessCompetition {
        id: Uint128,
        distribution: Vec<MemberShare<String>>,
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
    Competition { id: Uint128 },
    #[returns(Vec<CompetitionResponse<CompetitionExt>>)]
    Competitions {
        start_after: Option<Uint128>,
        limit: Option<u32>,
        status: Option<CompetitionStatus>,
    },
    #[returns(cosmwasm_std::Binary)]
    QueryExtension { msg: QueryExt },
    #[serde(skip)]
    #[returns(PhantomData<CompetitionExt>)]
    _Phantom(PhantomData<CompetitionExt>),
}

#[cw_serde]
pub enum HookDirection {
    Incoming,
    Outgoing,
}
