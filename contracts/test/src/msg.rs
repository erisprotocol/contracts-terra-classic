use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {

    #[returns(TaxResponse)]
    Tax {
        denom: String,
        amount: Uint128,
    }
}

#[cw_serde]
pub struct TaxResponse {
    pub tax: Uint128
}