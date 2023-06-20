#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_binary, Uint128};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, TaxResponse};
use classic_bindings::TerraQuery;
use eris::asset::{Asset, AssetInfo};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<TerraQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Tax { denom, amount } => to_binary(&tax(deps, denom, amount)?),
    }
}

fn tax(
    deps: Deps<TerraQuery>,
    denom: String,
    amount: Uint128
) -> StdResult<TaxResponse> {
    let asset = Asset {
        info: AssetInfo::NativeToken { 
            denom
        },
        amount
    };

    let tax = asset.compute_tax(&deps.querier).unwrap();
    Ok(TaxResponse { tax })
}

#[cfg(test)]
mod tests {
    use classic_test_tube::{Module, Account, Treasury, classic_proto::classic::treasury::{QueryTaxCapRequest, QueryTaxRateRequest}};
    use cosmwasm_std::{Uint128, Decimal};

    #[test]
    fn test_wasm_query_tax() {
        use classic_test_tube::TerraTestApp;
        use classic_test_tube::Wasm;
        use cosmwasm_std::Coin;
        use crate::msg::{InstantiateMsg, QueryMsg, TaxResponse};
        static DECIMAL_FRACTION: Uint128 = Uint128::new(1_000_000_000_000_000_000u128);

        let app = TerraTestApp::default();
        let wasm = Wasm::new(&app);
        let treasury = Treasury::new(&app);

        let accs = app
            .init_accounts(
                &[
                    Coin::new(1_000_000_000_000, "uluna"),
                    Coin::new(1_000_000_000_000, "uusd"),
                ],
                2,
            )
            .unwrap();
            
        let admin = &accs[0];

        // store wasm code
        let wasm_byte_code = std::fs::read("./testdata/test-aarch64.wasm").unwrap();
        let code_id = wasm
            .store_code(&wasm_byte_code, None, admin)
            .unwrap()
            .data
            .code_id;
        assert_eq!(code_id, 1);

        // initialize admins and check if the state is correct
        let contract_addr = wasm
            .instantiate(
                code_id,
                &InstantiateMsg {},
                Some(&admin.address()),
                None,
                &[],
                admin,
            )
            .unwrap()
            .data
            .address;

        // calculate tax from querying to chain
        let tax_cap = treasury.query_tax_cap(&QueryTaxCapRequest{
            denom: "uluna".to_string()
        }).unwrap();

        let tax_rate = treasury.query_tax_rate(&QueryTaxRateRequest{}).unwrap();
        let dec_tax_rate = Decimal::from_ratio(Uint128::new(tax_rate.tax_rate.parse::<u128>().unwrap()), DECIMAL_FRACTION);

        let amount = Uint128::new(1000000000);

        let tax = std::cmp::min(
            (amount.checked_sub(amount.multiply_ratio(
                DECIMAL_FRACTION,
                DECIMAL_FRACTION * dec_tax_rate + DECIMAL_FRACTION,
            ))).unwrap(),
            Uint128::new(tax_cap.tax_cap.parse::<u128>().unwrap()),
        );
        
        // query tax from contract
        let query = QueryMsg::Tax {
            denom: "uluna".to_string(),
            amount,
        };

        let res = wasm.query::<QueryMsg, TaxResponse>(contract_addr.as_str(), &query).unwrap();
        assert_eq!(tax, res.tax)
    }
}