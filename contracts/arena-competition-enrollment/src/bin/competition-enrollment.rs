use arena_competition_enrollment::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, SudoMsg};
use arena_interface::enrollments::QueryMsg;
use cosmwasm_schema::write_api;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
        migrate: MigrateMsg,
        sudo: SudoMsg
    }
}
