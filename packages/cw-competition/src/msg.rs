use std::marker::PhantomData;

#[allow(unused_imports)]
use crate::{
    core::CompetitionCoreActivateMsg,
    state::{CompetitionResponse, Config},
};
use cosmwasm_schema::{cw_serde, schemars::JsonSchema, QueryResponses};
use cosmwasm_std::Uint128;
use cw_balance::MemberShare;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use cw_utils::Expiration;
use dao_interface::state::ModuleInstantiateInfo;

#[cw_serde]
pub struct InstantiateBase<InstantiateExt> {
    pub key: String, //this is used to map a key (wager) to a module
    pub description: String,
    pub extension: InstantiateExt,
}

#[cw_ownable_execute]
#[cw_serde]
#[allow(clippy::large_enum_variant)]
pub enum ExecuteBase<ExecuteExt, CompetitionExt> {
    JailCompetition {
        id: Uint128,
        title: String,
        description: String,
    },
    Activate(CompetitionCoreActivateMsg),
    CreateCompetition {
        competition_dao: ModuleInstantiateInfo,
        escrow: ModuleInstantiateInfo,
        name: String,
        description: String,
        expiration: Expiration,
        rules: Vec<String>,
        ruleset: Option<Uint128>,
        extension: CompetitionExt,
    },
    GenerateProposals {
        id: Uint128,
        title: String,
        description: String,
    },
    ProcessCompetition {
        id: Uint128,
        distribution: Option<Vec<MemberShare>>,
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
    #[returns(Vec<(u128, CompetitionResponse<CompetitionExt>)>)]
    Competitions {
        start_after: Option<Uint128>,
        limit: Option<u32>,
    },
    #[returns(cosmwasm_std::Binary)]
    QueryExtension { msg: QueryExt },
    #[serde(skip)]
    #[returns(PhantomData<CompetitionExt>)]
    _Phantom(PhantomData<CompetitionExt>),
}
