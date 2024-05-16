use cosmwasm_std::{Addr, Coins};
use cw_multi_test::{App, AppBuilder, BankKeeper, MockAddressGenerator, MockApiBech32, WasmKeeper};

pub fn get_app() -> App<BankKeeper, MockApiBech32> {
    AppBuilder::default()
        .with_api(MockApiBech32::new("juno"))
        .with_wasm(WasmKeeper::default().with_address_generator(MockAddressGenerator))
        .build(|_, _, _| {})
}

pub fn set_balances(app: &mut App<BankKeeper, MockApiBech32>, balances: Vec<(Addr, Coins)>) {
    app.init_modules(|router, _, storage| {
        for balance in balances {
            router
                .bank
                .init_balance(storage, &balance.0, balance.1.into_vec())
                .unwrap();
        }
    });
}
