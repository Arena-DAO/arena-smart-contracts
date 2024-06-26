use std::str::FromStr;

use arena_interface::ratings::Rating;
use cosmwasm_std::{testing::mock_env, Decimal};
use cw_utils::Duration;
use rust_decimal_macros::dec;

use crate::rating::{
    calculate_new_sigma, expect_score, reduce_impact, update_rating_internal, RatingInternal, PHI,
    SIGMA,
};

#[test]
fn test_conversion_from_rating_to_internal() {
    let rating = Rating::new(
        Decimal::from_str("1500").unwrap(),
        Decimal::from_str("350").unwrap(),
        Decimal::from_str("0.06").unwrap(),
    );

    let rating_internal: RatingInternal = rating.into();

    assert_eq!(rating_internal.value, dec!(1500));
    assert_eq!(rating_internal.phi, dec!(350));
    assert_eq!(rating_internal.sigma, dec!(0.06));
}

#[test]
fn test_conversion_from_internal_to_rating() {
    let rating_internal = RatingInternal::new(dec!(1500), dec!(350), dec!(0.06));

    let rating: Rating = rating_internal.into();

    assert_eq!(rating.value, Decimal::from_str("1500").unwrap());
    assert_eq!(rating.phi, Decimal::from_str("350").unwrap());
    assert_eq!(rating.sigma, Decimal::from_str("0.06").unwrap());
}

#[test]
fn test_reduce_impact() {
    let impact = reduce_impact(PHI);
    assert_eq!(impact.round_dp(6), dec!(0.005156));
}

#[test]
fn test_expect_score() {
    let mu1 = dec!(1500);
    let mu2 = dec!(1600);
    let impact = dec!(0.005156);
    let expected_score = expect_score(mu1, mu2, impact);
    assert_eq!(expected_score.round_dp(6), dec!(0.373882));
}

#[test]
fn test_calculate_new_sigma() {
    let variance = dec!(0.9);
    let difference = dec!(0.5);
    let new_sigma = calculate_new_sigma(SIGMA, PHI, variance, difference);
    assert_eq!(new_sigma.round_dp(6), dec!(0.244949));
}

#[test]
fn test_update_rating() {
    let mut env = mock_env();
    let period = Duration::Height(10u64);
    let mut player1 = RatingInternal::new(dec!(1500), PHI, SIGMA);
    let mut player2 = RatingInternal::new(dec!(1500), PHI, SIGMA);
    let mut player3 = RatingInternal::new(dec!(1500), PHI, SIGMA);

    let win = dec!(1);
    let draw = dec!(0.5);
    let loss = dec!(0);

    update_rating_internal(&env, &mut player1, &mut player2, win, loss, &period);
    env.block.height += 10;

    assert_eq!(player1.value.round_dp(6), dec!(1507.725420));
    assert_eq!(player1.phi.round_dp(6), dec!(317.758916));
    assert_eq!(player1.sigma.round_dp(6), dec!(0.244782));

    assert_eq!(player2.value.round_dp(6), dec!(1492.274580));
    assert_eq!(player2.phi.round_dp(6), dec!(317.758916));
    assert_eq!(player2.sigma.round_dp(6), dec!(0.244782));

    // Run another round where player1 beats player 3
    update_rating_internal(&env, &mut player1, &mut player3, win, loss, &period);
    env.block.height += 10;

    // Expected values after the second match
    assert_eq!(player1.value.round_dp(6), dec!(1514.550284));
    assert_eq!(player1.phi.round_dp(6), dec!(300.266781));
    assert_eq!(player1.sigma.round_dp(6), dec!(0.493354));

    assert_eq!(player3.value.round_dp(6), dec!(1491.303415));
    assert_eq!(player3.phi.round_dp(6), dec!(314.691122));
    assert_eq!(player3.sigma.round_dp(6), dec!(0.244769));

    // Run a round between player2 and player 3 - draw to observe the impact of period adjustments
    update_rating_internal(&env, &mut player2, &mut player3, draw, draw, &period);

    // Expected values after the third match
    assert_eq!(player2.value.round_dp(6), dec!(1492.263433));
    assert_eq!(player2.phi.round_dp(6), dec!(297.399847));
    assert_eq!(player2.sigma.round_dp(6), dec!(0.493220));

    assert_eq!(player3.value.round_dp(6), dec!(1491.314205));
    assert_eq!(player3.phi.round_dp(6), dec!(295.467494));
    assert_eq!(player3.sigma.round_dp(6), dec!(0.493213));
}
