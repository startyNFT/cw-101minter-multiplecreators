use crate::error::ContractError;
use crate::msg::{
    AllCreatorStatsResponse, ConfigResponse, CreatorStatsResponse, ExecuteMsg, InstantiateMsg,
    MinterQueryMsg, MinterTokenInfoResponse, PendingBalanceResponse, QueryMsg, SudoMsg,
    TokenRoyaltiesResponse,
};
use crate::state::{Config, CONFIG, CREATOR_STATS, PENDING_BALANCE, TOKEN_ROYALTIES};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult, Uint128,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;

const CONTRACT_NAME: &str = "crates.io:royalty-splitter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_LIMIT: u32 = 30;
const MAX_LIMIT: u32 = 100;

// ============== INSTANTIATE ==============

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate royalty BPS
    if msg.royalty_bps > 10000 {
        return Err(ContractError::InvalidRoyaltyBps { bps: msg.royalty_bps });
    }

    let config = Config {
        admin: info.sender.clone(),
        minter: deps.api.addr_validate(&msg.minter)?,
        collection: deps.api.addr_validate(&msg.collection)?,
        royalty_bps: msg.royalty_bps,
    };

    CONFIG.save(deps.storage, &config)?;
    PENDING_BALANCE.save(deps.storage, &Uint128::zero())?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", info.sender)
        .add_attribute("minter", msg.minter)
        .add_attribute("collection", msg.collection)
        .add_attribute("royalty_bps", msg.royalty_bps.to_string()))
}

// ============== EXECUTE ==============

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateMinter { minter } => execute_update_minter(deps, info, minter),
        ExecuteMsg::UpdateCollection { collection } => {
            execute_update_collection(deps, info, collection)
        }
        ExecuteMsg::UpdateRoyaltyBps { royalty_bps } => {
            execute_update_royalty_bps(deps, info, royalty_bps)
        }
        ExecuteMsg::ManualDistribute { token_id, amount } => {
            execute_manual_distribute(deps, env, info, token_id, amount)
        }
        ExecuteMsg::EmergencyWithdraw { recipient, amount } => {
            execute_emergency_withdraw(deps, info, recipient, amount)
        }
    }
}

fn execute_update_minter(
    deps: DepsMut,
    info: MessageInfo,
    minter: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized(
            "Only admin can update minter".to_string(),
        ));
    }

    config.minter = deps.api.addr_validate(&minter)?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_minter")
        .add_attribute("minter", minter))
}

fn execute_update_collection(
    deps: DepsMut,
    info: MessageInfo,
    collection: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized(
            "Only admin can update collection".to_string(),
        ));
    }

    config.collection = deps.api.addr_validate(&collection)?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_collection")
        .add_attribute("collection", collection))
}

fn execute_update_royalty_bps(
    deps: DepsMut,
    info: MessageInfo,
    royalty_bps: u64,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized(
            "Only admin can update royalty".to_string(),
        ));
    }

    if royalty_bps > 10000 {
        return Err(ContractError::InvalidRoyaltyBps { bps: royalty_bps });
    }

    config.royalty_bps = royalty_bps;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_royalty_bps")
        .add_attribute("royalty_bps", royalty_bps.to_string()))
}

fn execute_manual_distribute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: u32,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized(
            "Only admin can manually distribute".to_string(),
        ));
    }

    // Check contract balance
    let balance = deps
        .querier
        .query_balance(&env.contract.address, "ustars")?;

    if balance.amount < amount {
        return Err(ContractError::InsufficientFunds {
            needed: amount.to_string(),
            available: balance.amount.to_string(),
        });
    }

    // Query minter for creator
    let creator = query_token_creator(&deps, &config.minter, token_id)?;

    // Update stats
    update_creator_stats(deps.storage, &creator, amount)?;
    update_token_royalties(deps.storage, token_id, amount)?;

    // Send funds to creator
    let send_msg = BankMsg::Send {
        to_address: creator.to_string(),
        amount: vec![Coin {
            denom: "ustars".to_string(),
            amount,
        }],
    };

    Ok(Response::new()
        .add_message(send_msg)
        .add_attribute("action", "manual_distribute")
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("creator", creator.to_string())
        .add_attribute("amount", amount.to_string()))
}

fn execute_emergency_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized(
            "Only admin can emergency withdraw".to_string(),
        ));
    }

    let recipient_addr = deps.api.addr_validate(&recipient)?;

    let send_msg = BankMsg::Send {
        to_address: recipient_addr.to_string(),
        amount: vec![Coin {
            denom: "ustars".to_string(),
            amount,
        }],
    };

    Ok(Response::new()
        .add_message(send_msg)
        .add_attribute("action", "emergency_withdraw")
        .add_attribute("recipient", recipient)
        .add_attribute("amount", amount.to_string()))
}

// ============== SUDO (Called by Marketplace) ==============

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::SaleHook {
            collection,
            token_id,
            price,
            seller: _,
            buyer: _,
        } => sudo_sale_hook(deps, env, collection, token_id, price),
    }
}

fn sudo_sale_hook(
    deps: DepsMut,
    env: Env,
    collection: String,
    token_id: u32,
    price: Coin,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only process sales from our collection
    if collection != config.collection.to_string() {
        // Not our collection, ignore silently
        return Ok(Response::new()
            .add_attribute("action", "sale_hook_ignored")
            .add_attribute("reason", "different_collection"));
    }

    // Calculate royalty amount
    // royalty = price * royalty_bps / 10000
    let royalty_amount = price
        .amount
        .multiply_ratio(config.royalty_bps, 10000u64);

    if royalty_amount.is_zero() {
        return Ok(Response::new()
            .add_attribute("action", "sale_hook")
            .add_attribute("token_id", token_id.to_string())
            .add_attribute("royalty", "0")
            .add_attribute("reason", "zero_royalty"));
    }

    // Check if we have enough funds (marketplace should have sent royalty to us)
    let balance = deps
        .querier
        .query_balance(&env.contract.address, &price.denom)?;

    if balance.amount < royalty_amount {
        // Not enough funds yet - store as pending and return success
        // The funds might arrive in a separate message
        let mut pending = PENDING_BALANCE.load(deps.storage)?;
        pending += royalty_amount;
        PENDING_BALANCE.save(deps.storage, &pending)?;

        return Ok(Response::new()
            .add_attribute("action", "sale_hook_pending")
            .add_attribute("token_id", token_id.to_string())
            .add_attribute("royalty_amount", royalty_amount.to_string())
            .add_attribute("reason", "insufficient_balance"));
    }

    // Query minter for token creator
    let creator = query_token_creator(&deps, &config.minter, token_id)?;

    // Update stats
    update_creator_stats(deps.storage, &creator, royalty_amount)?;
    update_token_royalties(deps.storage, token_id, royalty_amount)?;

    // Send royalty to creator
    let send_msg = BankMsg::Send {
        to_address: creator.to_string(),
        amount: vec![Coin {
            denom: price.denom.clone(),
            amount: royalty_amount,
        }],
    };

    Ok(Response::new()
        .add_message(send_msg)
        .add_attribute("action", "sale_hook")
        .add_attribute("collection", collection)
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("sale_price", price.amount.to_string())
        .add_attribute("royalty_amount", royalty_amount.to_string())
        .add_attribute("creator", creator.to_string()))
}

// ============== HELPERS ==============

fn query_token_creator(
    deps: &DepsMut,
    minter: &Addr,
    token_id: u32,
) -> Result<Addr, ContractError> {
    let query_msg = MinterQueryMsg::TokenInfo {
        token_id: token_id.to_string(),
    };

    let response: MinterTokenInfoResponse = deps
        .querier
        .query_wasm_smart(minter.to_string(), &query_msg)
        .map_err(|e| ContractError::MinterQueryFailed {
            token_id,
            reason: e.to_string(),
        })?;

    deps.api
        .addr_validate(&response.token_royalty.creator)
        .map_err(|e| ContractError::MinterQueryFailed {
            token_id,
            reason: e.to_string(),
        })
}

fn update_creator_stats(
    storage: &mut dyn cosmwasm_std::Storage,
    creator: &Addr,
    amount: Uint128,
) -> StdResult<()> {
    let mut stats = CREATOR_STATS
        .may_load(storage, creator)?
        .unwrap_or_default();

    stats.total_received += amount;
    stats.total_sales += 1;

    CREATOR_STATS.save(storage, creator, &stats)
}

fn update_token_royalties(
    storage: &mut dyn cosmwasm_std::Storage,
    token_id: u32,
    amount: Uint128,
) -> StdResult<()> {
    let current = TOKEN_ROYALTIES
        .may_load(storage, token_id)?
        .unwrap_or_default();

    TOKEN_ROYALTIES.save(storage, token_id, &(current + amount))
}

// ============== QUERY ==============

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::CreatorStats { creator } => to_json_binary(&query_creator_stats(deps, creator)?),
        QueryMsg::TokenRoyalties { token_id } => {
            to_json_binary(&query_token_royalties(deps, token_id)?)
        }
        QueryMsg::PendingBalance {} => to_json_binary(&query_pending_balance(deps)?),
        QueryMsg::AllCreatorStats { start_after, limit } => {
            to_json_binary(&query_all_creator_stats(deps, start_after, limit)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

fn query_creator_stats(deps: Deps, creator: String) -> StdResult<CreatorStatsResponse> {
    let creator_addr = deps.api.addr_validate(&creator)?;
    let stats = CREATOR_STATS
        .may_load(deps.storage, &creator_addr)?
        .unwrap_or_default();

    Ok(CreatorStatsResponse { creator, stats })
}

fn query_token_royalties(deps: Deps, token_id: u32) -> StdResult<TokenRoyaltiesResponse> {
    let total_royalties = TOKEN_ROYALTIES
        .may_load(deps.storage, token_id)?
        .unwrap_or_default();

    Ok(TokenRoyaltiesResponse {
        token_id,
        total_royalties,
    })
}

fn query_pending_balance(deps: Deps) -> StdResult<PendingBalanceResponse> {
    let pending = PENDING_BALANCE.load(deps.storage)?;
    Ok(PendingBalanceResponse { pending })
}

fn query_all_creator_stats(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllCreatorStatsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let start = start_after
        .map(|s| deps.api.addr_validate(&s))
        .transpose()?;
    let start_bound = start.as_ref().map(Bound::exclusive);

    let creators: Vec<CreatorStatsResponse> = CREATOR_STATS
        .range(deps.storage, start_bound, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (addr, stats) = item?;
            Ok(CreatorStatsResponse {
                creator: addr.to_string(),
                stats,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(AllCreatorStatsResponse { creators })
}
