use std::marker::PhantomData;

use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_std::{
    to_binary, Addr, Binary, Decimal, Deps, DepsMut, Empty, Env, Event, MessageInfo, Reply,
    Response, StdResult, SubMsg, Uint128,
};
use cw_balance::MemberShare;
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use cw_utils::{parse_reply_execute_data, parse_reply_instantiate_data};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    core::CompetitionCoreJailMsg,
    error::CompetitionError,
    escrow::CompetitionEscrowDistributeMsg,
    msg::{CoreQueryMsg, ExecuteBase, InstantiateBase, QueryBase},
    proposal::create_competition_proposals,
    state::{Competition, CompetitionStatus, Config},
};

pub const DAO_REPLY_ID: u64 = 1;
pub const ESCROW_REPLY_ID: u64 = 2;
pub const PROCESS_REPLY_ID: u64 = 3;

pub struct CompetitionModuleContract<InstantiateExt, ExecuteExt, QueryExt, CompetitionExt> {
    pub admin: Admin<'static>,
    pub config: Item<'static, Config>,
    pub competition_count: Item<'static, Uint128>,
    pub competitions: Map<'static, u128, Competition<CompetitionExt>>,
    pub temp_competition: Item<'static, u128>,

    instantiate_type: PhantomData<InstantiateExt>,
    execute_type: PhantomData<ExecuteExt>,
    query_type: PhantomData<QueryExt>,
}

impl<InstantiateExt, ExecuteExt, QueryExt, CompetitionExt>
    CompetitionModuleContract<InstantiateExt, ExecuteExt, QueryExt, CompetitionExt>
{
    const fn new(
        admin_key: &'static str,
        config_key: &'static str,
        competition_count_key: &'static str,
        competitions_key: &'static str,
        temp_competition_key: &'static str,
    ) -> Self {
        Self {
            admin: Admin::new(admin_key),
            config: Item::new(config_key),
            competition_count: Item::new(competition_count_key),
            competitions: Map::new(competitions_key),
            temp_competition: Item::new(temp_competition_key),
            instantiate_type: PhantomData,
            execute_type: PhantomData,
            query_type: PhantomData,
        }
    }
}

impl<InstantiateExt, ExecuteExt, QueryExt, CompetitionExt> Default
    for CompetitionModuleContract<InstantiateExt, ExecuteExt, QueryExt, CompetitionExt>
{
    fn default() -> Self {
        Self::new(
            "admin",
            "config",
            "competition_count",
            "competitions",
            "temp_competition",
        )
    }
}

impl<InstantiateExt, ExecuteExt, QueryExt, CompetitionExt>
    CompetitionModuleContract<InstantiateExt, ExecuteExt, QueryExt, CompetitionExt>
where
    CompetitionExt: Serialize + DeserializeOwned,
    QueryExt: JsonSchema,
{
    pub fn instantiate(
        &self,
        mut deps: DepsMut,
        info: MessageInfo,
        msg: InstantiateBase<InstantiateExt>,
    ) -> StdResult<Response> {
        self.config.save(
            deps.storage,
            &Config {
                key: msg.key.clone(),
                description: msg.description.clone(),
            },
        )?;
        self.admin.set(deps.branch(), Some(info.sender))?;
        self.competition_count
            .save(deps.storage, &Uint128::zero())?;

        Ok(Response::new()
            .add_attribute("key", msg.key)
            .add_attribute("description", msg.description))
    }

    pub fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteBase<ExecuteExt, CompetitionExt>,
    ) -> Result<Response, CompetitionError> {
        match msg {
            ExecuteBase::JailCompetition { id } => self.execute_jail_competition(deps, env, id),
            ExecuteBase::CreateCompetition {
                competition_dao,
                escrow,
                name,
                description,
                expiration,
                rules,
                ruleset,
                extension,
            } => self.execute_create_competition(
                deps,
                env,
                competition_dao,
                escrow,
                name,
                description,
                expiration,
                rules,
                ruleset,
                extension,
            ),
            ExecuteBase::ProcessCompetition { id, distribution } => {
                self.execute_process_competition(deps, info, id, distribution)
            }
            ExecuteBase::Extension { .. } => Ok(Response::default()),
        }
    }

    pub fn execute_jail_competition(
        &self,
        deps: DepsMut,
        env: Env,
        id: Uint128,
    ) -> Result<Response, CompetitionError> {
        let core = self.admin.get(deps.as_ref())?.unwrap();
        let competition = self.competitions.update(
            deps.storage,
            id.u128(),
            |x| -> Result<_, CompetitionError> {
                if x.is_none() {
                    return Err(CompetitionError::UnknownCompetitionId { id: id.u128() });
                }
                let mut competition = x.unwrap();

                if competition.status != CompetitionStatus::Active {
                    return Err(CompetitionError::InvalidCompetitionStatus {
                        current_status: competition.status,
                    });
                }
                if !competition.expiration.is_expired(&env.block) {
                    return Err(CompetitionError::CompetitionNotExpired {});
                }

                competition.status = CompetitionStatus::Jailed;

                Ok(competition)
            },
        )?;

        let msg = CompetitionCoreJailMsg { id: competition.id }.into_cosmos_msg(core)?;

        Ok(Response::new()
            .add_attribute("action", "jail_wager")
            .add_message(msg))
    }

    pub fn execute_create_competition(
        &self,
        deps: DepsMut,
        env: Env,
        competition_dao: dao_interface::ModuleInstantiateInfo,
        escrow: dao_interface::ModuleInstantiateInfo,
        name: String,
        description: String,
        expiration: cw_utils::Expiration,
        rules: Vec<String>,
        ruleset: Option<Uint128>,
        extension: CompetitionExt,
    ) -> Result<Response, CompetitionError> {
        let id = self
            .competition_count
            .update(deps.storage, |x| -> StdResult<_> {
                Ok(x.checked_add(Uint128::one())?)
            })?;
        let competition = Competition {
            start_height: env.block.height,
            id,
            dao: Addr::unchecked("temp"),
            escrow: Addr::unchecked("temp"),
            name,
            description,
            expiration,
            rules,
            ruleset,
            status: CompetitionStatus::Pending,
            extension,
        };
        let dao = self.query_dao(deps.as_ref())?;
        let msgs = vec![
            SubMsg::reply_always(competition_dao.into_wasm_msg(dao.clone()), DAO_REPLY_ID),
            SubMsg::reply_always(escrow.into_wasm_msg(dao.clone()), ESCROW_REPLY_ID),
        ];
        self.competitions
            .save(deps.storage, id.u128(), &competition)?;
        self.temp_competition.save(deps.storage, &id.u128())?;

        Ok(Response::new()
            .add_attribute("action", "create_competition")
            .add_attribute("id", id)
            .add_submessages(msgs))
    }

    pub fn execute_process_competition(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        id: Uint128,
        distribution: Option<Vec<cw_balance::MemberShare>>,
    ) -> Result<Response, CompetitionError> {
        let competition = match self.competitions.may_load(deps.storage, id.u128())? {
            Some(val) => Ok(val),
            None => Err(CompetitionError::UnknownCompetitionId { id: id.u128() }),
        }?;
        let dao = self.query_dao(deps.as_ref())?;

        match competition.status {
            CompetitionStatus::Active => {
                if competition.dao != info.sender && dao != info.sender {
                    return Err(CompetitionError::Unauthorized {});
                }
                Ok(())
            }
            CompetitionStatus::Jailed => {
                if dao != info.sender {
                    return Err(CompetitionError::Unauthorized {});
                }
                Ok(())
            }
            _ => Err(CompetitionError::InvalidCompetitionStatus {
                current_status: competition.status,
            }),
        }?;

        // Apply tax
        let distribution = match distribution {
            Some(mut member_shares) => {
                let arena_core = self.admin.get(deps.as_ref())?.unwrap();
                let tax: Decimal = deps.querier.query_wasm_smart(
                    arena_core,
                    &CoreQueryMsg::Tax {
                        height: Some(competition.start_height),
                    },
                )?;
                let sum = member_shares
                    .iter()
                    .try_fold(Uint128::zero(), |accumulator, x| {
                        accumulator.checked_add(x.shares)
                    })?;
                let dao_shares = tax
                    .checked_mul(Decimal::from_atomics(sum, 0u32)?)?
                    .checked_div(Decimal::one().checked_sub(tax)?)?;
                let dao_shares = dao_shares
                    .checked_div(Decimal::from_atomics(
                        Uint128::new(10u128).checked_pow(dao_shares.decimal_places())?,
                        0u32,
                    )?)?
                    .atomics();

                member_shares.push(MemberShare {
                    addr: dao.to_string(),
                    shares: dao_shares,
                });
                Some(member_shares)
            }
            None => None,
        };

        let sub_msg = SubMsg::reply_on_success(
            CompetitionEscrowDistributeMsg {
                distribution,
                remainder_addr: dao.to_string(),
            }
            .into_cosmos_msg(competition.escrow)?,
            PROCESS_REPLY_ID,
        );

        Ok(Response::new()
            .add_attribute("action", "process_competition")
            .add_submessage(sub_msg))
    }

    pub fn query(
        &self,
        deps: Deps,
        _env: Env,
        msg: QueryBase<QueryExt, CompetitionExt>,
    ) -> StdResult<Binary> {
        match msg {
            QueryBase::DAO {} => to_binary(&self.query_dao(deps)?),
            QueryBase::Config {} => to_binary(&self.config.load(deps.storage)?),
            QueryBase::Competition { id } => {
                to_binary(&self.competitions.load(deps.storage, id.u128())?)
            }
            QueryBase::Admin {} => to_binary(&self.admin.query_admin(deps)?),
            QueryBase::QueryExtension { .. } => Ok(Binary::default()),
            QueryBase::_Phantom(_) => Ok(Binary::default()),
        }
    }

    pub fn query_dao(&self, deps: Deps) -> StdResult<Addr> {
        let core = self.admin.get(deps)?.unwrap();
        let dao: Addr = deps
            .querier
            .query_wasm_smart(core, &dao_pre_propose_base::msg::QueryMsg::<Empty>::Dao {})?;

        Ok(dao)
    }

    pub fn reply(&self, deps: DepsMut, env: Env, msg: Reply) -> Result<Response, CompetitionError> {
        match msg.id {
            DAO_REPLY_ID => self.reply_dao(deps, env, msg),
            ESCROW_REPLY_ID => self.reply_escrow(deps, msg),
            PROCESS_REPLY_ID => self.reply_process(deps, msg),
            _ => Err(CompetitionError::UnknownReplyId { id: msg.id }),
        }
    }

    pub fn reply_dao(
        &self,
        deps: DepsMut,
        env: Env,
        msg: Reply,
    ) -> Result<Response, CompetitionError> {
        let result = parse_reply_instantiate_data(msg.clone())?;
        let addr = deps.api.addr_validate(&result.contract_address)?;
        let id = self.temp_competition.load(deps.storage)?;

        self.competitions
            .update(deps.storage, id, |x| -> Result<_, CompetitionError> {
                match x {
                    Some(mut competition) => {
                        competition.dao = addr.clone();
                        Ok(competition)
                    }
                    None => Err(CompetitionError::UnknownCompetitionId { id }),
                }
            })?;

        fn extract_attribute_value(
            events: &[Event],
            key: &str,
        ) -> Result<String, CompetitionError> {
            for event in events {
                for attribute in &event.attributes {
                    if attribute.key == key {
                        return Ok(attribute.value.clone());
                    }
                }
            }
            Err(CompetitionError::AttributeNotFound {
                key: key.to_string(),
            })
        }

        const GROUP_KEY: &str = "group_contract_address";
        const PROPOSAL_KEY: &str = "prop_module";
        let events = msg.result.unwrap().events;
        let cw4_group = deps
            .api
            .addr_validate(&extract_attribute_value(&events, GROUP_KEY)?)?;
        let proposal_module = deps
            .api
            .addr_validate(&extract_attribute_value(&events, PROPOSAL_KEY)?)?;

        Ok(Response::new()
            .add_attribute("action", "reply_dao")
            .add_attribute("id", Uint128::from(id))
            .add_attribute("dao_addr", addr.clone())
            .add_message(create_competition_proposals(
                deps.as_ref(),
                Uint128::from(id),
                &env.contract.address,
                &cw4_group,
                &proposal_module,
            )?))
    }

    pub fn reply_escrow(&self, deps: DepsMut, msg: Reply) -> Result<Response, CompetitionError> {
        let result = parse_reply_instantiate_data(msg)?;
        let addr = deps.api.addr_validate(&result.contract_address)?;
        let id = self.temp_competition.load(deps.storage)?;

        self.competitions
            .update(deps.storage, id, |x| -> Result<_, CompetitionError> {
                match x {
                    Some(mut competition) => {
                        competition.escrow = addr.clone();
                        Ok(competition)
                    }
                    None => Err(CompetitionError::UnknownCompetitionId { id }),
                }
            })?;

        Ok(Response::new()
            .add_attribute("action", "reply_escrow")
            .add_attribute("wager", Uint128::from(id))
            .add_attribute("escrow_addr", addr))
    }

    pub fn reply_process(&self, deps: DepsMut, msg: Reply) -> Result<Response, CompetitionError> {
        parse_reply_execute_data(msg)?;
        let id = self.temp_competition.load(deps.storage)?;

        self.competitions
            .update(deps.storage, id, |x| -> Result<_, CompetitionError> {
                match x {
                    Some(mut competition) => {
                        competition.status = CompetitionStatus::Inactive {};
                        Ok(competition)
                    }
                    None => Err(CompetitionError::UnknownCompetitionId { id }),
                }
            })?;

        Ok(Response::new()
            .add_attribute("action", "reply_process")
            .add_attribute("wager", Uint128::from(id)))
    }
}
