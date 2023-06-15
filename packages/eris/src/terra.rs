use cosmwasm_std::{Coin, CustomQuery, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TerraQueryWrapper {
    Swap {
        offer_coin: Coin,
        ask_denom: String,
    },
    TaxRate {},
    TaxCap {
        denom: String,
    },
    ExchangeRates {
        base_denom: String,
        quote_denoms: Vec<String>,
    },
}

/// TaxRateResponse is data format returned from TreasuryRequest::TaxRate query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TaxRateResponse {
    pub rate: Decimal,
}

/// TaxCapResponse is data format returned from TreasuryRequest::TaxCap query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TaxCapResponse {
    pub cap: Uint128,
}

impl CustomQuery for TerraQueryWrapper {}
