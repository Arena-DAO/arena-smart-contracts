# Glicko-2 Rating System

This package provides an implementation of the Glicko-2 rating system in Rust. The Glicko-2 rating system is a method for assessing a player's skill level in competitive games. It is an improvement over the original Glicko system and includes the concept of rating volatility.

## Features

- Calculation of player ratings based on match results
- Adjustment of rating deviations (\(\phi\)) based on periods of inactivity
- Calculation of new volatility (\(\sigma\)) after each rating period
- Supports flexible rating periods based on block height or time