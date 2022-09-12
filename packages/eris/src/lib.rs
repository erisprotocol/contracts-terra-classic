pub mod asset;
pub mod hub;

mod decimal_checked_ops {
    use cosmwasm_std::{Decimal, Decimal256, Fraction, OverflowError, StdError, Uint128, Uint256};
    use std::{convert::TryInto, str::FromStr};

    pub trait DecimalCheckedOps {
        // fn checked_add(self, other: Decimal) -> Result<Decimal, StdError>;
        fn checked_mul_uint(self, other: Uint128) -> Result<Uint128, StdError>;
        fn to_decimal256(self) -> Decimal256;
    }

    impl DecimalCheckedOps for Decimal {
        // fn checked_add(self, other: Decimal) -> Result<Decimal, StdError> {
        //     self.numerator()
        //         .checked_add(other.numerator())
        //         .map(|_| self + other)
        //         .map_err(StdError::overflow)
        // }

        fn checked_mul_uint(self, other: Uint128) -> Result<Uint128, StdError> {
            if self.is_zero() || other.is_zero() {
                return Ok(Uint128::zero());
            }
            let multiply_ratio =
                other.full_mul(self.numerator()) / Uint256::from(self.denominator());
            if multiply_ratio > Uint256::from(Uint128::MAX) {
                Err(StdError::overflow(OverflowError::new(
                    cosmwasm_std::OverflowOperation::Mul,
                    self,
                    other,
                )))
            } else {
                Ok(multiply_ratio.try_into().unwrap())
            }
        }

        fn to_decimal256(self) -> Decimal256 {
            Decimal256::from_str(&self.to_string()).unwrap()
        }
    }
}

pub use decimal_checked_ops::DecimalCheckedOps;
