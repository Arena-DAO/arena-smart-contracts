use crate::{BalanceError, GenericTokenBalance};
use cosmwasm_std::{OverflowError, OverflowOperation};
use std::collections::HashSet;

pub trait GenericBalanceExtensions {
    fn get_hashset(&self) -> HashSet<GenericTokenBalance>;
    fn add_balances_checked(
        &self,
        to_add: &Vec<GenericTokenBalance>,
    ) -> Result<Vec<GenericTokenBalance>, BalanceError>;
    fn sub_balances_checked(
        &self,
        to_sub: &Vec<GenericTokenBalance>,
    ) -> Result<Vec<GenericTokenBalance>, BalanceError>;
}

impl GenericBalanceExtensions for Vec<GenericTokenBalance> {
    fn get_hashset(&self) -> HashSet<GenericTokenBalance> {
        self.iter().cloned().collect()
    }

    fn add_balances_checked(
        &self,
        to_add: &Vec<GenericTokenBalance>,
    ) -> Result<Vec<GenericTokenBalance>, BalanceError> {
        let mut result = vec![];
        let mut set = self.get_hashset();

        for token in to_add {
            match set.get(token) {
                Some(val) => {
                    result.push(val.checked_add(token)?);
                    set.remove(token);
                }
                None => {
                    result.push(token.clone());
                }
            }
        }

        for token in set {
            result.push(token);
        }

        Ok(result)
    }

    fn sub_balances_checked(
        &self,
        to_sub: &Vec<GenericTokenBalance>,
    ) -> Result<Vec<GenericTokenBalance>, BalanceError> {
        let mut result = vec![];
        let mut set = self.get_hashset();

        for token in to_sub {
            match set.get(token) {
                Some(val) => {
                    let sub = val.checked_sub(token)?;
                    if !sub.amount.is_zero() {
                        result.push(val.checked_sub(token)?);
                    }
                    set.remove(token);
                }
                None => {
                    return Err(BalanceError::Overflow(OverflowError::new(
                        OverflowOperation::Sub,
                        "NULL",
                        token.to_string(),
                    )));
                }
            }
        }

        for token in set {
            result.push(token);
        }

        Ok(result)
    }
}
