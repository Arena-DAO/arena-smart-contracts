use std::f64::consts::PI;

use arena_interface::ratings::Rating;
use cosmwasm_std::{BlockInfo, Decimal as StdDecimal, Env};
use cw_utils::Duration;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;

use super::util::{rust_to_std, std_to_rust};

#[derive(Clone, Debug, PartialEq)]
pub struct RatingInternal {
    pub value: Decimal,
    pub phi: Decimal,
    pub sigma: Decimal,
    pub last_block: Option<BlockInfo>,
}

impl RatingInternal {
    pub fn new(mu: Decimal, phi: Decimal, sigma: Decimal) -> Self {
        Self {
            value: mu,
            phi,
            sigma,
            last_block: None,
        }
    }
}

pub const DEFAULT_RATING: Decimal = dec!(1500);
#[allow(dead_code)]
pub const PHI: Decimal = dec!(350);
#[allow(dead_code)]
pub const SIGMA: Decimal = dec!(0.06);
pub const TAU: Decimal = Decimal::ONE;
pub const EPSILON: Decimal = dec!(0.000001);
pub const SCALING_FACTOR: Decimal = dec!(173.7178);

pub fn reduce_impact(phi: Decimal) -> Decimal {
    Decimal::ONE
        / (Decimal::ONE
            + (dec!(3) * phi * phi / Decimal::from_f64(PI * PI).unwrap())
                .sqrt()
                .unwrap())
}

pub fn expect_score(mu: Decimal, other_mu: Decimal, impact: Decimal) -> Decimal {
    Decimal::ONE / (Decimal::ONE + ((other_mu - mu) * impact).exp())
}

/// Calculates the periods based on the previous and current block info.
pub fn calculate_periods(
    env: &Env,
    previous_block_info: &BlockInfo,
    duration: &Duration,
) -> Decimal {
    match duration {
        Duration::Height(duration_blocks) => {
            Decimal::from_u64(env.block.height - previous_block_info.height).unwrap()
                / Decimal::from_u64(*duration_blocks).unwrap()
        }
        Duration::Time(duration_seconds) => {
            Decimal::from_u64(env.block.time.seconds() - previous_block_info.time.seconds())
                .unwrap()
                / Decimal::from_u64(*duration_seconds).unwrap()
        }
    }
}

/// Adjusts the rating deviation (phi) for the periods of inactivity.
pub fn adjust_phi_for_periods(phi: Decimal, sigma: Decimal, periods: Decimal) -> Decimal {
    (phi.powu(2) + sigma.powu(2) * periods).sqrt().unwrap()
}

/// Calculates the new volatility (sigma) of the rating.
pub fn calculate_new_sigma(
    sigma: Decimal,
    phi: Decimal,
    variance: Decimal,
    difference: Decimal,
) -> Decimal {
    let alpha = sigma.ln();
    let mut a = alpha;
    let mut b: Decimal;
    let f = |x: Decimal| {
        let tmp = phi.powu(2) + variance + x.exp();
        (x.exp() * (difference.powu(2) - tmp) / (Decimal::TWO * tmp.powu(2)))
            - ((x - alpha) / TAU.powu(2))
    };

    if difference.powu(2) > phi.powu(2) + variance {
        b = (difference.powu(2) - phi.powu(2) - variance).ln();
    } else {
        let mut k = Decimal::ONE;
        while f(alpha - k * TAU) < Decimal::ZERO {
            k += Decimal::ONE;
        }
        b = a - k * TAU;
    }

    let mut f_a = f(a);
    let mut f_b = f(b);

    while (b - a).abs() > EPSILON {
        let c = a + (a - b) * f_a / (f_b - f_a);
        let f_c = f(c);
        if f_c * f_b < Decimal::ZERO {
            a = b;
            f_a = f_b;
        } else {
            f_a /= Decimal::TWO;
        }
        b = c;
        f_b = f_c;
    }

    (a / Decimal::TWO).exp()
}

/// Updates the ratings of two players based on the match results and the periods of inactivity.
pub fn update_rating_internal(
    env: &Env,
    rating1: &mut RatingInternal,
    rating2: &mut RatingInternal,
    result1: Decimal,
    result2: Decimal,
    period: &Duration,
) {
    if let Some(last_block1) = &rating1.last_block {
        let periods1 = calculate_periods(env, last_block1, period);
        rating1.phi = adjust_phi_for_periods(rating1.phi, rating1.sigma, periods1);
    }

    if let Some(last_block2) = &rating2.last_block {
        let periods2 = calculate_periods(env, last_block2, period);
        rating2.phi = adjust_phi_for_periods(rating2.phi, rating2.sigma, periods2);
    }

    // Scaling down
    let mu1 = (rating1.value - DEFAULT_RATING) / SCALING_FACTOR;
    let phi1 = rating1.phi / SCALING_FACTOR;

    let mu2 = (rating2.value - DEFAULT_RATING) / SCALING_FACTOR;
    let phi2 = rating2.phi / SCALING_FACTOR;

    // Expected scores
    let impact1 = reduce_impact(phi2);
    let expected_score1 = expect_score(mu1, mu2, impact1);

    let impact2 = reduce_impact(phi1);
    let expected_score2 = expect_score(mu2, mu1, impact2);

    // Variance and difference
    let variance_inv1 = impact1 * impact1 * expected_score1 * (Decimal::ONE - expected_score1);
    let difference1 = impact1 * (result1 - expected_score1);
    let variance1 = Decimal::ONE / variance_inv1;

    let variance_inv2 = impact2 * impact2 * expected_score2 * (Decimal::ONE - expected_score2);
    let difference2 = impact2 * (result2 - expected_score2);
    let variance2 = Decimal::ONE / variance_inv2;

    // New volatility
    let new_sigma1 = calculate_new_sigma(rating1.sigma, phi1, variance1, difference1);
    let new_sigma2 = calculate_new_sigma(rating2.sigma, phi2, variance2, difference2);

    // Update rating and deviation
    let phi1_star = (phi1 * phi1 + new_sigma1 * new_sigma1).sqrt().unwrap();
    let phi2_star = (phi2 * phi2 + new_sigma2 * new_sigma2).sqrt().unwrap();

    let phi1_new = Decimal::ONE
        / ((Decimal::ONE / phi1_star / phi1_star) + (Decimal::ONE / variance1))
            .sqrt()
            .unwrap();
    let mu1_new = mu1 + phi1_new * phi1_new * (difference1 / variance1);

    let phi2_new = Decimal::ONE
        / ((Decimal::ONE / phi2_star / phi2_star) + (Decimal::ONE / variance2))
            .sqrt()
            .unwrap();
    let mu2_new = mu2 + phi2_new * phi2_new * (difference2 / variance2);

    // Scaling up
    rating1.value = mu1_new * SCALING_FACTOR + DEFAULT_RATING;
    rating1.phi = phi1_new * SCALING_FACTOR;
    rating1.sigma = new_sigma1;

    rating2.value = mu2_new * SCALING_FACTOR + DEFAULT_RATING;
    rating2.phi = phi2_new * SCALING_FACTOR;
    rating2.sigma = new_sigma2;

    rating1.last_block = Some(env.block.clone());
    rating2.last_block = Some(env.block.clone());
}

pub fn update_rating(
    env: &Env,
    rating1: &mut Rating,
    rating2: &mut Rating,
    result1: StdDecimal,
    result2: StdDecimal,
    period: &Duration,
) {
    // Convert Rating to internal representation
    let mut rating1_internal: RatingInternal = rating1.clone().into();
    let mut rating2_internal: RatingInternal = rating2.clone().into();

    // Update the internal ratings
    update_rating_internal(
        env,
        &mut rating1_internal,
        &mut rating2_internal,
        std_to_rust(result1),
        std_to_rust(result2),
        period,
    );

    // Convert back to Rating
    *rating1 = rating1_internal.into();
    *rating2 = rating2_internal.into();
}

// Conversion from Rating to RatingInternal
impl From<Rating> for RatingInternal {
    fn from(r: Rating) -> Self {
        RatingInternal {
            value: std_to_rust(r.value),
            phi: std_to_rust(r.phi),
            sigma: std_to_rust(r.sigma),
            last_block: r.last_block,
        }
    }
}

// Conversion from RatingInternal to Rating
impl From<RatingInternal> for Rating {
    fn from(r: RatingInternal) -> Self {
        Rating {
            value: rust_to_std(r.value),
            phi: rust_to_std(r.phi),
            sigma: rust_to_std(r.sigma),
            last_block: r.last_block,
        }
    }
}
