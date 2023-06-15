pub mod asset;
pub mod hub;

mod extensions {
    use cosmwasm_std::{
        CosmosMsg, Decimal, Decimal256, Empty, Env, Fraction, OverflowError, Response, StdError,
        StdResult, Uint128, Uint256,
    };
    use std::{convert::TryInto, str::FromStr};

    use crate::hub::CallbackMsg;

    pub trait CustomResponse<T>: Sized {
        fn add_optional_message(self, msg: Option<CosmosMsg<T>>) -> Self;
        fn add_callback_message(self, env: &Env, msg: CallbackMsg) -> StdResult<Self>;
    }

    impl CustomResponse<Empty> for Response {
        fn add_optional_message(self, msg: Option<CosmosMsg>) -> Self {
            match msg {
                Some(msg) => self.add_message(msg),
                None => self,
            }
        }

        fn add_callback_message(self, env: &Env, msg: CallbackMsg) -> StdResult<Self> {
            Ok(self.add_message(msg.into_cosmos_msg(&env.contract.address)?))
        }
    }

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

pub use extensions::CustomResponse;
pub use extensions::DecimalCheckedOps;
