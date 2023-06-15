use cosmwasm_std::{
    entry_point, from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError, StdResult,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20ReceiveMsg;
use eris::terra::TerraQueryWrapper;

use eris::hub::{CallbackMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ReceiveMsg};

use crate::constants::{CONTRACT_NAME, CONTRACT_VERSION};
use crate::helpers::{parse_received_fund, unwrap_reply};
use crate::state::State;
use crate::{execute, queries};

#[entry_point]
pub fn instantiate(
    deps: DepsMut<TerraQueryWrapper>,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    execute::instantiate(deps, env, msg)
}

#[entry_point]
pub fn execute(
    deps: DepsMut<TerraQueryWrapper>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    let api = deps.api;
    match msg {
        ExecuteMsg::Receive(cw20_msg) => receive(deps, env, info, cw20_msg),
        ExecuteMsg::Bond {
            receiver,
        } => execute::bond(
            deps,
            env,
            receiver.map(|s| api.addr_validate(&s)).transpose()?.unwrap_or(info.sender),
            parse_received_fund(&info.funds, "uluna")?,
            false,
        ),
        ExecuteMsg::Donate {} => {
            execute::bond(deps, env, info.sender, parse_received_fund(&info.funds, "uluna")?, true)
        },
        ExecuteMsg::WithdrawUnbonded {
            receiver,
        } => execute::withdraw_unbonded(
            deps,
            env,
            info.sender.clone(),
            receiver.map(|s| api.addr_validate(&s)).transpose()?.unwrap_or(info.sender),
        ),
        ExecuteMsg::AddValidator {
            validator,
        } => execute::add_validator(deps, info.sender, validator),
        ExecuteMsg::RemoveValidator {
            validator,
        } => execute::remove_validator(deps, env, info.sender, validator),
        ExecuteMsg::TransferOwnership {
            new_owner,
        } => execute::transfer_ownership(deps, info.sender, new_owner),
        ExecuteMsg::AcceptOwnership {} => execute::accept_ownership(deps, info.sender),
        ExecuteMsg::Harvest {} => execute::harvest(deps, env),
        ExecuteMsg::Rebalance {} => execute::rebalance(deps, env),
        ExecuteMsg::Reconcile {} => execute::reconcile(deps, env),
        ExecuteMsg::SubmitBatch {} => execute::submit_batch(deps, env),
        ExecuteMsg::Callback(callback_msg) => callback(deps, env, info, callback_msg),
        ExecuteMsg::UpdateConfig {
            protocol_fee_contract,
            protocol_reward_fee,
            swap_config,
        } => execute::update_config(
            deps,
            info.sender,
            protocol_fee_contract,
            protocol_reward_fee,
            swap_config,
        ),
    }
}

fn receive(
    deps: DepsMut<TerraQueryWrapper>,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response> {
    let api = deps.api;
    match from_binary(&cw20_msg.msg)? {
        ReceiveMsg::QueueUnbond {
            receiver,
        } => {
            let state = State::default();

            let stake_token = state.stake_token.load(deps.storage)?;
            if info.sender != stake_token {
                return Err(StdError::generic_err(format!(
                    "expecting Stake token, received {}",
                    info.sender
                )));
            }

            execute::queue_unbond(
                deps,
                env,
                api.addr_validate(&receiver.unwrap_or(cw20_msg.sender))?,
                cw20_msg.amount,
            )
        },
    }
}

fn callback(
    deps: DepsMut<TerraQueryWrapper>,
    env: Env,
    info: MessageInfo,
    callback_msg: CallbackMsg,
) -> StdResult<Response> {
    if env.contract.address != info.sender {
        return Err(StdError::generic_err("callbacks can only be invoked by the contract itself"));
    }

    match callback_msg {
        CallbackMsg::Swap {} => execute::swap(deps, env),
        CallbackMsg::Reinvest {} => execute::reinvest(deps, env),
        CallbackMsg::CheckReceivedCoin {
            snapshot,
        } => execute::callback_received_coin(deps, env, snapshot),
    }
}

#[entry_point]
pub fn reply(deps: DepsMut<TerraQueryWrapper>, _env: Env, reply: Reply) -> StdResult<Response> {
    match reply.id {
        1 => execute::register_stake_token(deps, unwrap_reply(reply)?),
        id => Err(StdError::generic_err(format!("invalid reply id: {}; must be 1", id))),
    }
}

#[entry_point]
pub fn query(deps: Deps<TerraQueryWrapper>, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::config(deps)?),
        QueryMsg::State {} => to_binary(&queries::state(deps, env)?),
        QueryMsg::PendingBatch {} => to_binary(&queries::pending_batch(deps)?),
        QueryMsg::PreviousBatch(id) => to_binary(&queries::previous_batch(deps, id)?),
        QueryMsg::PreviousBatches {
            start_after,
            limit,
        } => to_binary(&queries::previous_batches(deps, start_after, limit)?),
        QueryMsg::UnbondRequestsByBatch {
            id,
            start_after,
            limit,
        } => to_binary(&queries::unbond_requests_by_batch(deps, id, start_after, limit)?),
        QueryMsg::UnbondRequestsByUser {
            user,
            start_after,
            limit,
        } => to_binary(&queries::unbond_requests_by_user(deps, user, start_after, limit)?),

        QueryMsg::UnbondRequestsByUserDetails {
            user,
            start_after,
            limit,
        } => to_binary(&queries::unbond_requests_by_user_details(
            deps,
            user,
            start_after,
            limit,
            env,
        )?),
    }
}

#[entry_point]
pub fn migrate(
    deps: DepsMut<TerraQueryWrapper>,
    _env: Env,
    _msg: MigrateMsg,
) -> StdResult<Response> {
    let contract_version = get_contract_version(deps.storage)?;

    match contract_version.contract.as_ref() {
        "eris-staking-hub" => match contract_version.version.as_ref() {
            "1.1.0" => {
                let state = State::default();
                state.swap_config.save(deps.storage, &vec![])?;
            },
            "1.1.1" => {},
            "1.1.2" => {},
            "1.2.0" => {},
            "1.2.1" => {},
            "1.2.2" => {},
            _ => return Err(StdError::generic_err("Error during migration")),
        },
        _ => return Err(StdError::generic_err("Error during migration")),
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new())
}
