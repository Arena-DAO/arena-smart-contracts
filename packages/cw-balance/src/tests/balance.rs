use crate::BalanceVerified;

#[test]
fn test_add_empty_balances() {
    let balance_a = BalanceVerified::default();
    let balance_b = BalanceVerified::default();

    let new_balance = balance_a.checked_add(&balance_b).unwrap();
    assert!(new_balance.is_empty());
}

#[test]
fn test_subtract_empty_balances() {
    let balance_a = BalanceVerified::default();
    let balance_b = BalanceVerified::default();

    let new_balance = balance_a.checked_sub(&balance_b).unwrap();
    assert!(new_balance.is_empty());
}
