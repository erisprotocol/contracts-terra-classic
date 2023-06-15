use classic_bindings::{TerraQuerier, TerraQuery};
// Code is adjusted based on https://github.com/astroport-fi/astroport-core/blob/release/terra1/packages/astroport/src/asset.rs
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use cosmwasm_std::{
    to_binary, Addr, Api, BankMsg, Coin, CosmosMsg, Decimal, MessageInfo, QuerierWrapper, StdError,
    StdResult, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PairExecuteMsg {
    /// Swap an offer asset to the other
    Swap {
        offer_asset: Asset,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
    },
}

/// UST token denomination
pub const UUSD_DENOM: &str = "uusd";
/// LUNA token denomination
pub const ULUNA_DENOM: &str = "uluna";

/// ## Description
/// This enum describes a Terra asset (native or CW20).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Asset {
    /// Information about an asset stored in a [`AssetInfo`] struct
    pub info: AssetInfo,
    /// A token amount
    pub amount: Uint128,
}

impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.amount, self.info)
    }
}

/// Decimal points
static DECIMAL_FRACTION: Uint128 = Uint128::new(1_000_000_000_000_000_000u128);

impl Asset {
    /// Returns true if the token is native. Otherwise returns false.
    /// ## Params
    /// * **self** is the type of the caller object.
    pub fn is_native_token(&self) -> bool {
        self.info.is_native_token()
    }

    /// Calculates and returns a tax for a chain's native token. For other tokens it returns zero.
    /// ## Params
    /// * **self** is the type of the caller object.
    ///
    /// * **querier** is an object of type [`QuerierWrapper`]
    pub fn compute_tax(&self, querier: &QuerierWrapper<TerraQuery>) -> StdResult<Uint128> {
        let amount = self.amount;
        if let AssetInfo::NativeToken {
            denom,
        } = &self.info
        {
            // https://terra-classic-lcd.publicnode.com/cosmos/params/v1beta1/params?subspace=treasury&key=TaxPolicy
            let querier = TerraQuerier::new(querier);
            let tax_rate: Decimal = querier.query_tax_rate()?.rate;
            let tax_cap: Uint128 = querier.query_tax_cap(denom)?.cap;

            // let tax_rate: Decimal = Decimal::from_str("0.005")?;
            // let tax_cap: Uint128 = Uint128::MAX;

            Ok(std::cmp::min(
                (amount.checked_sub(amount.multiply_ratio(
                    DECIMAL_FRACTION,
                    DECIMAL_FRACTION * tax_rate + DECIMAL_FRACTION,
                )))?,
                tax_cap,
            ))
        } else {
            Ok(Uint128::zero())
        }
    }

    /// Calculates and returns a deducted tax for transferring the native token from the chain. For other tokens it returns an [`Err`].
    /// ## Params
    /// * **self** is the type of the caller object.
    ///
    /// * **querier** is an object of type [`QuerierWrapper`]
    pub fn deduct_tax(&self, querier: &QuerierWrapper<TerraQuery>) -> StdResult<Coin> {
        let amount = self.amount;
        if let AssetInfo::NativeToken {
            denom,
        } = &self.info
        {
            Ok(Coin {
                denom: denom.to_string(),
                amount: amount.checked_sub(self.compute_tax(querier)?)?,
            })
        } else {
            Err(StdError::generic_err("cannot deduct tax from token asset"))
        }
    }

    /// Returns a message of type [`CosmosMsg`].
    ///
    /// For native tokens of type [`AssetInfo`] uses the default method [`BankMsg::Send`] to send a token amount to a recipient.
    /// Before the token is sent, we need to deduct a tax.
    ///
    /// For a token of type [`AssetInfo`] we use the default method [`Cw20ExecuteMsg::Transfer`] and so there's no need to deduct any other tax.
    /// ## Params
    /// * **self** is the type of the caller object.
    ///
    /// * **querier** is an object of type [`QuerierWrapper`]
    ///
    /// * **recipient** is the address where the funds will be sent.
    pub fn into_msg(
        self,
        querier: &QuerierWrapper<TerraQuery>,
        recipient: Addr,
    ) -> StdResult<CosmosMsg> {
        let amount = self.amount;

        match &self.info {
            AssetInfo::Token {
                contract_addr,
            } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: recipient.to_string(),
                    amount,
                })?,
                funds: vec![],
            })),
            AssetInfo::NativeToken {
                ..
            } => Ok(CosmosMsg::Bank(BankMsg::Send {
                to_address: recipient.to_string(),
                amount: vec![self.deduct_tax(querier)?],
            })),
        }
    }

    pub fn into_swap_msg(
        self,
        querier: &QuerierWrapper<TerraQuery>,
        pair_contract: String,
        max_spread: Option<Decimal>,
        to: Option<String>,
    ) -> StdResult<CosmosMsg> {
        match &self.info {
            AssetInfo::NativeToken {
                denom,
            } => {
                // Deduct tax first
                let amount = self.amount.checked_sub(self.compute_tax(querier)?)?;
                Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: pair_contract,
                    funds: vec![Coin {
                        denom: denom.to_string(),
                        amount,
                    }],
                    msg: to_binary(&PairExecuteMsg::Swap {
                        offer_asset: Asset {
                            amount,
                            ..self
                        },
                        belief_price: None,
                        max_spread,
                        to,
                    })?,
                }))
            },
            AssetInfo::Token {
                contract_addr,
            } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: pair_contract,
                    amount: self.amount,
                    msg: to_binary(&PairExecuteMsg::Swap {
                        offer_asset: self,
                        belief_price: None,
                        max_spread,
                        to,
                    })?,
                })?,
            })),
        }
    }

    /// Validates an amount of native tokens being sent. Returns [`Ok`] if successful, otherwise returns [`Err`].
    /// ## Params
    /// * **self** is the type of the caller object.
    ///
    /// * **message_info** is an object of type [`MessageInfo`]
    pub fn assert_sent_native_token_balance(&self, message_info: &MessageInfo) -> StdResult<()> {
        if let AssetInfo::NativeToken {
            denom,
        } = &self.info
        {
            match message_info.funds.iter().find(|x| x.denom == *denom) {
                Some(coin) => {
                    if self.amount == coin.amount {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance mismatch between the argument and the transferred"))
                    }
                },
                None => {
                    if self.amount.is_zero() {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance mismatch between the argument and the transferred"))
                    }
                },
            }
        } else {
            Ok(())
        }
    }
}

/// This enum describes available Token types.
/// ## Examples
/// ```
/// # use cosmwasm_std::Addr;
/// # use astroport::asset::AssetInfo::{NativeToken, Token};
/// Token { contract_addr: Addr::unchecked("terra...") };
/// NativeToken { denom: String::from("uluna") };
/// ```
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetInfo {
    /// Non-native Token
    Token {
        contract_addr: Addr,
    },
    /// Native token
    NativeToken {
        denom: String,
    },
}

impl fmt::Display for AssetInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AssetInfo::NativeToken {
                denom,
            } => write!(f, "{}", denom),
            AssetInfo::Token {
                contract_addr,
            } => write!(f, "{}", contract_addr),
        }
    }
}

impl AssetInfo {
    /// Returns true if the caller is a native token. Otherwise returns false.
    /// ## Params
    /// * **self** is the caller object type
    pub fn is_native_token(&self) -> bool {
        match self {
            AssetInfo::NativeToken {
                ..
            } => true,
            AssetInfo::Token {
                ..
            } => false,
        }
    }

    /// Returns True if the calling token is the same as the token specified in the input parameters.
    /// Otherwise returns False.
    /// ## Params
    /// * **self** is the type of the caller object.
    ///
    /// * **asset** is object of type [`AssetInfo`].
    pub fn equal(&self, asset: &AssetInfo) -> bool {
        match self {
            AssetInfo::Token {
                contract_addr,
                ..
            } => {
                let self_contract_addr = contract_addr;
                match asset {
                    AssetInfo::Token {
                        contract_addr,
                        ..
                    } => self_contract_addr == contract_addr,
                    AssetInfo::NativeToken {
                        ..
                    } => false,
                }
            },
            AssetInfo::NativeToken {
                denom,
                ..
            } => {
                let self_denom = denom;
                match asset {
                    AssetInfo::Token {
                        ..
                    } => false,
                    AssetInfo::NativeToken {
                        denom,
                        ..
                    } => self_denom == denom,
                }
            },
        }
    }

    /// If the caller object is a native token of type ['AssetInfo`] then his `denom` field converts to a byte string.
    ///
    /// If the caller object is a token of type ['AssetInfo`] then his `contract_addr` field converts to a byte string.
    /// ## Params
    /// * **self** is the type of the caller object.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            AssetInfo::NativeToken {
                denom,
            } => denom.as_bytes(),
            AssetInfo::Token {
                contract_addr,
            } => contract_addr.as_bytes(),
        }
    }

    /// Returns [`Ok`] if the token of type [`AssetInfo`] is in lowercase and valid. Otherwise returns [`Err`].
    /// ## Params
    /// * **self** is the type of the caller object.
    ///
    /// * **api** is a object of type [`Api`]
    pub fn check(&self, api: &dyn Api) -> StdResult<()> {
        match self {
            AssetInfo::Token {
                contract_addr,
            } => {
                addr_validate_to_lower(api, contract_addr.as_str())?;
            },
            AssetInfo::NativeToken {
                denom,
            } => {
                if !denom.starts_with("ibc/") && denom != &denom.to_lowercase() {
                    return Err(StdError::generic_err(format!(
                        "Non-IBC token denom {} should be lowercase",
                        denom
                    )));
                }
            },
        }
        Ok(())
    }
}

/// Returns a lowercased, validated address upon success. Otherwise returns [`Err`]
/// ## Params
/// * **api** is an object of type [`Api`]
///
/// * **addr** is an object of type [`Addr`]
pub fn addr_validate_to_lower(api: &dyn Api, addr: &str) -> StdResult<Addr> {
    if addr.to_lowercase() != addr {
        return Err(StdError::generic_err(format!("Address {} should be lowercase", addr)));
    }
    api.addr_validate(addr)
}

/// Returns an [`Asset`] object representing a native token and an amount of tokens.
/// ## Params
/// * **denom** is a [`String`] that represents the native asset denomination.
///
/// * **amount** is a [`Uint128`] representing an amount of native assets.
pub fn native_asset(denom: String, amount: Uint128) -> Asset {
    Asset {
        info: AssetInfo::NativeToken {
            denom,
        },
        amount,
    }
}

/// Returns an [`Asset`] object representing a non-native token and an amount of tokens.
/// ## Params
/// * **contract_addr** is a [`Addr`]. It is the address of the token contract.
///
/// * **amount** is a [`Uint128`] representing an amount of tokens.
pub fn token_asset(contract_addr: Addr, amount: Uint128) -> Asset {
    Asset {
        info: AssetInfo::Token {
            contract_addr,
        },
        amount,
    }
}

/// Returns an [`AssetInfo`] object representing the denomination for a Terra native asset.
/// ## Params
/// * **denom** is a [`String`] object representing the denomination of the Terra native asset.
pub fn native_asset_info(denom: String) -> AssetInfo {
    AssetInfo::NativeToken {
        denom,
    }
}

/// Returns an [`AssetInfo`] object representing the address of a token contract.
/// ## Params
/// * **contract_addr** is a [`Addr`] object representing the address of a token contract.
pub fn token_asset_info(contract_addr: Addr) -> AssetInfo {
    AssetInfo::Token {
        contract_addr,
    }
}
