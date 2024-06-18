use arena_escrow::msg::{InstantiateMsg, MigrateMsg};
use arena_interface::escrow::{ExecuteMsg, QueryMsg};
use cosmwasm_schema::write_api;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
        migrate: MigrateMsg
    }
}
