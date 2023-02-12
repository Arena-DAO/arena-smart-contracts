use cosmwasm_schema::write_api;
use cw4_disbursement::msg::{ExecuteMsg, MigrateMsg, QueryMsg};
use cw4_group::msg::InstantiateMsg;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
        migrate: MigrateMsg
    }
}
