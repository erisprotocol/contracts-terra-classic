use cosmwasm_std::{Decimal, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::{Bound, U64Key};

use eris::hub::{
    Batch, ConfigResponse, PendingBatch, StateResponse, UnbondRequestsByBatchResponseItem,
    UnbondRequestsByUserResponseItem, UnbondRequestsByUserResponseItemDetails,
};

use crate::helpers::{query_cw20_total_supply, query_delegations};
use crate::state::State;

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = State::default();
    Ok(ConfigResponse {
        owner: state.owner.load(deps.storage)?.into(),
        new_owner: state.new_owner.may_load(deps.storage)?.map(|addr| addr.into()),
        stake_token: state.stake_token.load(deps.storage)?.into(),
        epoch_period: state.epoch_period.load(deps.storage)?,
        unbond_period: state.unbond_period.load(deps.storage)?,
        validators: state.validators.load(deps.storage)?,
        fee_config: state.fee_config.load(deps.storage)?,
        swap_config: state.swap_config.load(deps.storage)?,
    })
}

pub fn state(deps: Deps, env: Env) -> StdResult<StateResponse> {
    let state = State::default();

    let stake_token = state.stake_token.load(deps.storage)?;
    let total_ustake = query_cw20_total_supply(&deps.querier, &stake_token)?;

    let validators = state.validators.load(deps.storage)?;
    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let total_uluna: u128 = delegations.iter().map(|d| d.amount).sum();

    // only not reconciled batches are relevant as they are still unbonding and estimated unbond time in the future.
    let unbonding: u128 = state
        .previous_batches
        .idx
        .reconciled
        .prefix(false.into())
        .range(deps.storage, None, None, Order::Descending)
        .map(|item| {
            let (_, v) = item.unwrap();
            v
        })
        .filter(|item| item.est_unbond_end_time > env.block.time.seconds())
        .map(|item| item.uluna_unclaimed.u128())
        .sum();

    let available = deps.querier.query_balance(&env.contract.address, "uluna")?.amount;

    let exchange_rate = if total_ustake.is_zero() {
        Decimal::one()
    } else {
        Decimal::from_ratio(total_uluna, total_ustake)
    };

    Ok(StateResponse {
        total_ustake,
        total_uluna: Uint128::new(total_uluna),
        exchange_rate,
        unlocked_coins: state.unlocked_coins.load(deps.storage)?,
        unbonding: Uint128::from(unbonding),
        available,
        tvl_uluna: Uint128::from(total_uluna)
            .checked_add(Uint128::from(unbonding))?
            .checked_add(available)?,
    })
}

pub fn pending_batch(deps: Deps) -> StdResult<PendingBatch> {
    let state = State::default();
    state.pending_batch.load(deps.storage)
}

pub fn previous_batch(deps: Deps, id: u64) -> StdResult<Batch> {
    let state = State::default();
    state.previous_batches.load(deps.storage, id.into())
}

pub fn previous_batches(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<Batch>> {
    let state = State::default();

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|id| Bound::exclusive(U64Key::from(id)));

    state
        .previous_batches
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;
            Ok(v)
        })
        .collect()
}

pub fn unbond_requests_by_batch(
    deps: Deps,
    id: u64,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<UnbondRequestsByBatchResponseItem>> {
    let state = State::default();

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let start = match start_after {
        None => None,
        Some(addr_str) => Some(Bound::exclusive(deps.api.addr_validate(&addr_str)?.into_string())),
    };

    state
        .unbond_requests
        .prefix(id.into())
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;
            Ok(v.into())
        })
        .collect()
}

pub fn unbond_requests_by_user(
    deps: Deps,
    user: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<UnbondRequestsByUserResponseItem>> {
    let state = State::default();

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = deps.api.addr_validate(&user)?;

    let start = start_after.map(|id| {
        let mut key = vec![0u8, 8u8]; // when `u64` are used as keys, they are prefixed with the length, which is [0, 8]
        key.extend(id.to_be_bytes());
        key.extend(addr.to_string().as_bytes().to_vec());
        Bound::exclusive(key)
    });

    state
        .unbond_requests
        .idx
        .user
        .prefix(user)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;
            Ok(v.into())
        })
        .collect()
}

pub fn unbond_requests_by_user_details(
    deps: Deps,
    user: String,
    start_after: Option<u64>,
    limit: Option<u32>,
    env: Env,
) -> StdResult<Vec<UnbondRequestsByUserResponseItemDetails>> {
    let state = State::default();

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = deps.api.addr_validate(&user)?;
    let start = start_after.map(|id| {
        let mut key = vec![0u8, 8u8]; // when `u64` are used as keys, they are prefixed with the length, which is [0, 8]
        key.extend(id.to_be_bytes());
        key.extend(addr.to_string().as_bytes().to_vec());
        Bound::exclusive(key)
    });
    let pending = state.pending_batch.load(deps.storage)?;

    state
        .unbond_requests
        .idx
        .user
        .prefix(user)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;

            let state_msg: String;
            let previous: Option<Batch>;
            if pending.id == v.id {
                state_msg = "PENDING".to_string();
                previous = None;
            } else {
                let batch = state.previous_batches.load(deps.storage, v.id.into())?;
                previous = Some(batch.clone());
                let current_time = env.block.time.seconds();
                state_msg = if batch.est_unbond_end_time < current_time {
                    "COMPLETED".to_string()
                } else {
                    "UNBONDING".to_string()
                }
            }

            Ok(UnbondRequestsByUserResponseItemDetails {
                id: v.id,
                shares: v.shares,
                state: state_msg,
                pending: if pending.id == v.id {
                    Some(pending.clone())
                } else {
                    None
                },
                batch: previous,
            })
        })
        .collect()
}
