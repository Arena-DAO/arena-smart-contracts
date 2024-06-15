use std::{cmp::min, str::FromStr};

use cosmwasm_std::Decimal as StdDecimal;
use rust_decimal::{prelude::ToPrimitive, Decimal as RustDecimal};

pub fn rust_to_std(x: RustDecimal) -> StdDecimal {
    // Determine the scale to preserve precision, up to 9 decimal places
    let digits = min(x.scale(), 9);
    let multiplier = 10u128.pow(digits);

    // Multiply up to avoid floating-point errors, then convert to u128
    let numerator = (x * RustDecimal::new(multiplier as i64, 0))
        .round()
        .to_u128()
        .unwrap();

    // Create StdDecimal using the ratio of numerator to multiplier
    StdDecimal::from_ratio(numerator, multiplier)
}

pub fn std_to_rust(x: StdDecimal) -> RustDecimal {
    // Convert StdDecimal to string and then parse it into RustDecimal
    RustDecimal::from_str(&x.to_string()).unwrap()
}
