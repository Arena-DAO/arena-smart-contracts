use std::collections::HashMap;

use crate::Arena;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, CosmosMsg, WasmMsg};
use cw_orch::{anyhow, prelude::*};
use dao_proposal_sudo::msg::ExecuteMsgFns as _;

pub fn setup_arena(mock: &MockBech32) -> anyhow::Result<(Arena<MockBech32>, Addr)> {
    let admin = mock.addr_make(crate::tests::ADMIN);
    let arena = Arena::deploy_on(mock.clone(), admin.clone())?;
    mock.next_block()?;
    Ok((arena, admin))
}

pub fn setup_vesting(
    arena: &Arena<MockBech32>,
    chain_id: String,
    admin: &Addr,
) -> anyhow::Result<()> {
    // Set up the payroll widget
    arena.dao_dao.cw_payroll_factory.instantiate(
        &cw_payroll_factory::msg::InstantiateMsg {
            owner: Some(arena.dao_dao.dao_core.addr_str()?),
            vesting_code_id: arena.dao_dao.cw_vesting.code_id()?,
        },
        Some(&arena.dao_dao.dao_core.address()?),
        None,
    )?;

    let item_value = serde_json::to_string(&PayrollData {
        factories: Factories {
            chain_factories: [(
                chain_id,
                PayrollFactory {
                    address: arena.dao_dao.cw_payroll_factory.addr_str()?,
                    version: 2,
                },
            )]
            .into_iter()
            .collect(),
        },
    })?;
    arena
        .dao_dao
        .dao_proposal_sudo
        .call_as(admin)
        .proposal_execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: arena.dao_dao.dao_core.addr_str()?,
            msg: to_json_binary(&dao_interface::msg::ExecuteMsg::SetItem {
                key: "widget:vesting".to_string(),
                value: item_value,
            })?,
            funds: vec![],
        })])?;

    Ok(())
}

#[cw_serde]
pub struct PayrollFactory {
    pub address: String,
    pub version: u32,
}

#[cw_serde]
pub struct Factories {
    #[serde(flatten)]
    pub chain_factories: HashMap<String, PayrollFactory>,
}

#[cw_serde]
pub struct PayrollData {
    pub factories: Factories,
}
