use crate::error::ContractError;
use crate::msg::{
    AllNftInfoResponse, CheckRoyaltiesResponse, CollectionInfoResponse, ContractInfoResponse,
    ExecuteMsg, GeneralRoyaltyInfo, GeneralRoyaltyInfoResponse, InstantiateMsg, MinterResponse,
    NftInfoResponse, QueryMsg, RoyaltyInfoResponse, TokenExtension,
};
use crate::state::{
    CollectionConfig, TokenInfo, COLLECTION_INFO, CONFIG, GENERAL_ROYALTY, OPERATORS,
    OWNER_TOKENS, TOKENS, TOKEN_APPROVALS, TOKEN_COUNT,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult, Uint128,
};
use cw2::set_contract_version;
use cw721::receiver::Cw721ReceiveMsg;
use cw721::Expiration;
use cw_storage_plus::Bound;

const CONTRACT_NAME: &str = "crates.io:multi-creator-collection";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_LIMIT: u32 = 30;
const MAX_LIMIT: u32 = 100;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let minter = deps.api.addr_validate(&msg.minter)?;
    let creator = msg
        .creator
        .map(|c| deps.api.addr_validate(&c))
        .transpose()?
        .or_else(|| Some(info.sender.clone()));

    let config = CollectionConfig {
        name: msg.name.clone(),
        symbol: msg.symbol.clone(),
        minter: Some(minter.clone()),
        creator,
    };

    CONFIG.save(deps.storage, &config)?;
    COLLECTION_INFO.save(deps.storage, &msg.collection_info)?;
    GENERAL_ROYALTY.save(deps.storage, &msg.general_royalty)?;
    TOKEN_COUNT.save(deps.storage, &0u64)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("name", msg.name)
        .add_attribute("symbol", msg.symbol)
        .add_attribute("minter", minter))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Mint {
            token_id,
            owner,
            token_uri,
            extension,
        } => execute_mint(deps, info, token_id, owner, token_uri, extension),
        ExecuteMsg::TransferNft {
            recipient,
            token_id,
        } => execute_transfer(deps, env, info, recipient, token_id),
        ExecuteMsg::SendNft {
            contract,
            token_id,
            msg,
        } => execute_send(deps, env, info, contract, token_id, msg),
        ExecuteMsg::Approve {
            spender,
            token_id,
            expires,
        } => execute_approve(deps, env, info, spender, token_id, expires),
        ExecuteMsg::Revoke { spender, token_id } => execute_revoke(deps, env, info, spender, token_id),
        ExecuteMsg::ApproveAll { operator, expires } => {
            execute_approve_all(deps, env, info, operator, expires)
        }
        ExecuteMsg::RevokeAll { operator } => execute_revoke_all(deps, info, operator),
        ExecuteMsg::Burn { token_id } => execute_burn(deps, env, info, token_id),
        ExecuteMsg::UpdateMinter { new_minter } => execute_update_minter(deps, info, new_minter),
        ExecuteMsg::UpdateGeneralRoyalty { general_royalty } => {
            execute_update_general_royalty(deps, info, general_royalty)
        }
    }
}

fn execute_mint(
    deps: DepsMut,
    info: MessageInfo,
    token_id: String,
    owner: String,
    token_uri: Option<String>,
    extension: TokenExtension,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only minter can mint
    if config.minter != Some(info.sender.clone()) {
        return Err(ContractError::Unauthorized("Only minter can mint".to_string()));
    }

    // Check token doesn't exist
    if TOKENS.has(deps.storage, &token_id) {
        return Err(ContractError::TokenAlreadyExists { token_id });
    }

    let owner_addr = deps.api.addr_validate(&owner)?;

    let token_info = TokenInfo {
        owner: owner_addr.clone(),
        token_uri: token_uri.clone(),
        extension,
    };

    TOKENS.save(deps.storage, &token_id, &token_info)?;

    // Add to owner index for O(1) lookups
    OWNER_TOKENS.save(deps.storage, (&owner_addr, &token_id), &())?;

    // Increment token count (optimized: use update instead of load+save)
    TOKEN_COUNT.update(deps.storage, |count| Ok::<_, ContractError>(count + 1))?;

    Ok(Response::new()
        .add_attribute("action", "mint")
        .add_attribute("token_id", token_id)
        .add_attribute("owner", owner_addr))
}

fn execute_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    token_id: String,
) -> Result<Response, ContractError> {
    let recipient_addr = deps.api.addr_validate(&recipient)?;
    _transfer(deps, env, info, recipient_addr.clone(), token_id.clone())?;

    Ok(Response::new()
        .add_attribute("action", "transfer_nft")
        .add_attribute("token_id", token_id)
        .add_attribute("recipient", recipient_addr))
}

fn execute_send(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract: String,
    token_id: String,
    msg: Binary,
) -> Result<Response, ContractError> {
    let contract_addr = deps.api.addr_validate(&contract)?;
    _transfer(deps, env, info.clone(), contract_addr.clone(), token_id.clone())?;

    let send_msg = Cw721ReceiveMsg {
        sender: info.sender.to_string(),
        token_id: token_id.clone(),
        msg,
    };

    Ok(Response::new()
        .add_message(send_msg.into_cosmos_msg(contract_addr.clone())?)
        .add_attribute("action", "send_nft")
        .add_attribute("token_id", token_id)
        .add_attribute("recipient", contract_addr))
}

fn _transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Addr,
    token_id: String,
) -> Result<(), ContractError> {
    let mut token = TOKENS.load(deps.storage, &token_id).map_err(|_| {
        ContractError::TokenNotFound {
            token_id: token_id.clone(),
        }
    })?;

    // Check authorization
    check_can_send(deps.as_ref(), &env, &info, &token, &token_id)?;

    let old_owner = token.owner.clone();

    // Update owner index: remove from old owner, add to new owner
    OWNER_TOKENS.remove(deps.storage, (&old_owner, &token_id));
    OWNER_TOKENS.save(deps.storage, (&recipient, &token_id), &())?;

    // Clear all token approvals (optimized: iterate and remove)
    let approvals_to_remove: Vec<Addr> = TOKEN_APPROVALS
        .prefix(&token_id)
        .keys(deps.storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .collect();
    for spender in approvals_to_remove {
        TOKEN_APPROVALS.remove(deps.storage, (&token_id, &spender));
    }

    // Update owner
    token.owner = recipient;
    TOKENS.save(deps.storage, &token_id, &token)?;

    Ok(())
}

fn check_can_send(
    deps: Deps,
    env: &Env,
    info: &MessageInfo,
    token: &TokenInfo,
    token_id: &str,
) -> Result<(), ContractError> {
    // Owner can always send
    if token.owner == info.sender {
        return Ok(());
    }

    // Check operator approval
    if let Some(exp) = OPERATORS.may_load(deps.storage, (&token.owner, &info.sender))? {
        if !exp.is_expired(&env.block) {
            return Ok(());
        }
    }

    // Check token-level approval (optimized: direct lookup instead of iteration)
    if let Some(exp) = TOKEN_APPROVALS.may_load(deps.storage, (token_id, &info.sender))? {
        if !exp.is_expired(&env.block) {
            return Ok(());
        }
    }

    Err(ContractError::NotOwnerOrApproved {})
}

fn execute_approve(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    spender: String,
    token_id: String,
    expires: Option<Expiration>,
) -> Result<Response, ContractError> {
    let spender_addr = deps.api.addr_validate(&spender)?;
    let token = TOKENS.load(deps.storage, &token_id).map_err(|_| {
        ContractError::TokenNotFound {
            token_id: token_id.clone(),
        }
    })?;

    // Only owner can approve
    if token.owner != info.sender {
        return Err(ContractError::Unauthorized(
            "Only owner can approve".to_string(),
        ));
    }

    // Save approval to separate storage (optimized: no need to load/save entire token)
    let exp = expires.unwrap_or(Expiration::Never {});
    TOKEN_APPROVALS.save(deps.storage, (&token_id, &spender_addr), &exp)?;

    Ok(Response::new()
        .add_attribute("action", "approve")
        .add_attribute("token_id", token_id)
        .add_attribute("spender", spender_addr))
}

fn execute_revoke(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    spender: String,
    token_id: String,
) -> Result<Response, ContractError> {
    let spender_addr = deps.api.addr_validate(&spender)?;
    let token = TOKENS.load(deps.storage, &token_id).map_err(|_| {
        ContractError::TokenNotFound {
            token_id: token_id.clone(),
        }
    })?;

    // Only owner can revoke
    if token.owner != info.sender {
        return Err(ContractError::Unauthorized(
            "Only owner can revoke".to_string(),
        ));
    }

    // Remove from separate approvals storage (optimized: no need to load/save entire token)
    TOKEN_APPROVALS.remove(deps.storage, (&token_id, &spender_addr));

    Ok(Response::new()
        .add_attribute("action", "revoke")
        .add_attribute("token_id", token_id)
        .add_attribute("spender", spender_addr))
}

fn execute_approve_all(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    operator: String,
    expires: Option<Expiration>,
) -> Result<Response, ContractError> {
    let operator_addr = deps.api.addr_validate(&operator)?;
    let exp = expires.unwrap_or(Expiration::Never {});
    OPERATORS.save(deps.storage, (&info.sender, &operator_addr), &exp)?;

    Ok(Response::new()
        .add_attribute("action", "approve_all")
        .add_attribute("operator", operator_addr)
        .add_attribute("sender", info.sender))
}

fn execute_revoke_all(
    deps: DepsMut,
    info: MessageInfo,
    operator: String,
) -> Result<Response, ContractError> {
    let operator_addr = deps.api.addr_validate(&operator)?;
    OPERATORS.remove(deps.storage, (&info.sender, &operator_addr));

    Ok(Response::new()
        .add_attribute("action", "revoke_all")
        .add_attribute("operator", operator_addr)
        .add_attribute("sender", info.sender))
}

fn execute_burn(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let token = TOKENS.load(deps.storage, &token_id).map_err(|_| {
        ContractError::TokenNotFound {
            token_id: token_id.clone(),
        }
    })?;

    // Only owner can burn
    if token.owner != info.sender {
        return Err(ContractError::Unauthorized(
            "Only owner can burn".to_string(),
        ));
    }

    // Remove from owner index
    OWNER_TOKENS.remove(deps.storage, (&token.owner, &token_id));

    // Clear all token approvals
    let approvals_to_remove: Vec<Addr> = TOKEN_APPROVALS
        .prefix(&token_id)
        .keys(deps.storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .collect();
    for spender in approvals_to_remove {
        TOKEN_APPROVALS.remove(deps.storage, (&token_id, &spender));
    }

    TOKENS.remove(deps.storage, &token_id);

    // Decrement token count (optimized: use update)
    TOKEN_COUNT.update(deps.storage, |count| Ok::<_, ContractError>(count.saturating_sub(1)))?;

    Ok(Response::new()
        .add_attribute("action", "burn")
        .add_attribute("token_id", token_id))
}

fn execute_update_minter(
    deps: DepsMut,
    info: MessageInfo,
    new_minter: Option<String>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // Only current minter can update
    if config.minter != Some(info.sender.clone()) {
        return Err(ContractError::Unauthorized(
            "Only minter can update minter".to_string(),
        ));
    }

    config.minter = new_minter
        .map(|m| deps.api.addr_validate(&m))
        .transpose()?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_minter")
        .add_attribute(
            "new_minter",
            config.minter.map(|a| a.to_string()).unwrap_or_default(),
        ))
}

fn execute_update_general_royalty(
    deps: DepsMut,
    info: MessageInfo,
    general_royalty: GeneralRoyaltyInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only creator/admin can update general royalty
    if config.creator != Some(info.sender.clone()) {
        return Err(ContractError::Unauthorized(
            "Only creator can update general royalty".to_string(),
        ));
    }

    // Validate royalty address
    deps.api.addr_validate(&general_royalty.address)?;

    // Validate royalty doesn't exceed 100%
    if general_royalty.royalty_bps > 10000 {
        return Err(ContractError::Unauthorized(
            "Royalty cannot exceed 100%".to_string(),
        ));
    }

    GENERAL_ROYALTY.save(deps.storage, &Some(general_royalty.clone()))?;

    Ok(Response::new()
        .add_attribute("action", "update_general_royalty")
        .add_attribute("address", general_royalty.address)
        .add_attribute("royalty_bps", general_royalty.royalty_bps.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::RoyaltyInfo {
            token_id,
            sale_price,
        } => to_json_binary(&query_royalty_info(deps, token_id, sale_price)?),
        QueryMsg::CheckRoyalties {} => to_json_binary(&CheckRoyaltiesResponse {
            royalty_payments: true,
        }),
        QueryMsg::GeneralRoyalty {} => to_json_binary(&query_general_royalty(deps)?),
        QueryMsg::NftInfo { token_id } => to_json_binary(&query_nft_info(deps, token_id)?),
        QueryMsg::AllNftInfo {
            token_id,
            include_expired,
        } => to_json_binary(&query_all_nft_info(deps, env, token_id, include_expired)?),
        QueryMsg::OwnerOf {
            token_id,
            include_expired,
        } => to_json_binary(&query_owner_of(deps, env, token_id, include_expired)?),
        QueryMsg::AllOperators {
            owner,
            include_expired,
            start_after,
            limit,
        } => to_json_binary(&query_all_operators(
            deps,
            env,
            owner,
            include_expired,
            start_after,
            limit,
        )?),
        QueryMsg::NumTokens {} => to_json_binary(&query_num_tokens(deps)?),
        QueryMsg::ContractInfo {} => to_json_binary(&query_contract_info(deps)?),
        QueryMsg::Tokens {
            owner,
            start_after,
            limit,
        } => to_json_binary(&query_tokens(deps, owner, start_after, limit)?),
        QueryMsg::AllTokens { start_after, limit } => {
            to_json_binary(&query_all_tokens(deps, start_after, limit)?)
        }
        QueryMsg::Minter {} => to_json_binary(&query_minter(deps)?),
        QueryMsg::CollectionInfo {} => to_json_binary(&query_collection_info(deps)?),
        QueryMsg::Approval {
            token_id,
            spender,
            include_expired,
        } => to_json_binary(&query_approval(deps, env, token_id, spender, include_expired)?),
        QueryMsg::Approvals {
            token_id,
            include_expired,
        } => to_json_binary(&query_approvals(deps, env, token_id, include_expired)?),
    }
}

fn query_royalty_info(
    deps: Deps,
    token_id: Option<String>,
    sale_price: Uint128,
) -> StdResult<RoyaltyInfoResponse> {
    let general_royalty = GENERAL_ROYALTY.load(deps.storage)?;

    // If token_id is provided, check for token-specific royalty first
    if let Some(tid) = token_id {
        let token = TOKENS.load(deps.storage, &tid)?;

        // Check if token has specific royalty info (both creator and royalty_bps must be set)
        if token.extension.creator.is_some() && token.extension.royalty_bps.is_some() {
            let creator = token.extension.creator.unwrap();
            let royalty_bps = token.extension.royalty_bps.unwrap();

            let royalty_amount = if royalty_bps > 0 {
                let royalty_decimal = Decimal::bps(royalty_bps);
                sale_price.mul_floor(royalty_decimal)
            } else {
                Uint128::zero()
            };

            return Ok(RoyaltyInfoResponse {
                address: creator,
                royalty_amount,
            });
        }
    }

    // Fallback to general royalty
    match general_royalty {
        Some(gr) => {
            let royalty_amount = if gr.royalty_bps > 0 {
                let royalty_decimal = Decimal::bps(gr.royalty_bps);
                sale_price.mul_floor(royalty_decimal)
            } else {
                Uint128::zero()
            };

            Ok(RoyaltyInfoResponse {
                address: gr.address,
                royalty_amount,
            })
        }
        None => {
            // No royalty configured
            Ok(RoyaltyInfoResponse {
                address: String::new(),
                royalty_amount: Uint128::zero(),
            })
        }
    }
}

fn query_general_royalty(deps: Deps) -> StdResult<GeneralRoyaltyInfoResponse> {
    let general_royalty = GENERAL_ROYALTY.load(deps.storage)?;
    Ok(GeneralRoyaltyInfoResponse { general_royalty })
}

fn query_nft_info(deps: Deps, token_id: String) -> StdResult<NftInfoResponse> {
    let token = TOKENS.load(deps.storage, &token_id)?;
    Ok(NftInfoResponse {
        token_uri: token.token_uri,
        extension: token.extension,
    })
}

fn query_all_nft_info(
    deps: Deps,
    env: Env,
    token_id: String,
    include_expired: Option<bool>,
) -> StdResult<AllNftInfoResponse> {
    let token = TOKENS.load(deps.storage, &token_id)?;
    let include_expired = include_expired.unwrap_or(false);

    // Get approvals from separate storage
    let approvals: Vec<cw721::Approval> = TOKEN_APPROVALS
        .prefix(&token_id)
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .filter(|(_, exp)| include_expired || !exp.is_expired(&env.block))
        .map(|(spender, exp)| cw721::Approval { spender, expires: exp })
        .collect();

    Ok(AllNftInfoResponse {
        access: cw721::msg::OwnerOfResponse {
            owner: token.owner.to_string(),
            approvals,
        },
        info: NftInfoResponse {
            token_uri: token.token_uri,
            extension: token.extension,
        },
    })
}

fn query_owner_of(
    deps: Deps,
    env: Env,
    token_id: String,
    include_expired: Option<bool>,
) -> StdResult<cw721::msg::OwnerOfResponse> {
    let token = TOKENS.load(deps.storage, &token_id)?;
    let include_expired = include_expired.unwrap_or(false);

    // Get approvals from separate storage
    let approvals: Vec<cw721::Approval> = TOKEN_APPROVALS
        .prefix(&token_id)
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .filter(|(_, exp)| include_expired || !exp.is_expired(&env.block))
        .map(|(spender, exp)| cw721::Approval { spender, expires: exp })
        .collect();

    Ok(cw721::msg::OwnerOfResponse {
        owner: token.owner.to_string(),
        approvals,
    })
}

fn query_all_operators(
    deps: Deps,
    env: Env,
    owner: String,
    include_expired: Option<bool>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<cw721::msg::OperatorsResponse> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let include_expired = include_expired.unwrap_or(false);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let start = start_after
        .map(|s| deps.api.addr_validate(&s))
        .transpose()?;
    let start_bound = start.as_ref().map(Bound::exclusive);

    let operators: Vec<cw721::Approval> = OPERATORS
        .prefix(&owner_addr)
        .range(deps.storage, start_bound, None, Order::Ascending)
        .filter(|r| {
            include_expired
                || r.as_ref()
                    .map(|(_, exp)| !exp.is_expired(&env.block))
                    .unwrap_or(false)
        })
        .take(limit)
        .filter_map(|r| r.ok())
        .map(|(addr, exp)| cw721::Approval {
            spender: addr,
            expires: exp,
        })
        .collect();

    Ok(cw721::msg::OperatorsResponse { operators })
}

fn query_num_tokens(deps: Deps) -> StdResult<cw721::msg::NumTokensResponse> {
    let count = TOKEN_COUNT.load(deps.storage)?;
    Ok(cw721::msg::NumTokensResponse { count })
}

fn query_contract_info(deps: Deps) -> StdResult<ContractInfoResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ContractInfoResponse {
        name: config.name,
        symbol: config.symbol,
    })
}

fn query_tokens(
    deps: Deps,
    owner: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<cw721::msg::TokensResponse> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.as_deref().map(Bound::exclusive);

    // Optimized: Use owner index for O(1) lookups instead of scanning all tokens
    let tokens: Vec<String> = OWNER_TOKENS
        .prefix(&owner_addr)
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .filter_map(|r| r.ok())
        .collect();

    Ok(cw721::msg::TokensResponse { tokens })
}

fn query_all_tokens(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<cw721::msg::TokensResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.as_deref().map(Bound::exclusive);

    let tokens: Vec<String> = TOKENS
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .filter_map(|r| r.ok())
        .collect();

    Ok(cw721::msg::TokensResponse { tokens })
}

fn query_minter(deps: Deps) -> StdResult<MinterResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(MinterResponse {
        minter: config.minter.map(|a| a.to_string()),
    })
}

fn query_collection_info(deps: Deps) -> StdResult<CollectionInfoResponse> {
    let config = CONFIG.load(deps.storage)?;
    let extension = COLLECTION_INFO.load(deps.storage)?;
    Ok(CollectionInfoResponse {
        name: config.name,
        symbol: config.symbol,
        extension,
    })
}

fn query_approval(
    deps: Deps,
    env: Env,
    token_id: String,
    spender: String,
    include_expired: Option<bool>,
) -> StdResult<cw721::msg::ApprovalResponse> {
    // Verify token exists
    let _ = TOKENS.load(deps.storage, &token_id)?;
    let spender_addr = deps.api.addr_validate(&spender)?;
    let include_expired = include_expired.unwrap_or(false);

    // Get approval from separate storage (optimized: direct lookup)
    let approval = match TOKEN_APPROVALS.may_load(deps.storage, (&token_id, &spender_addr))? {
        Some(exp) if include_expired || !exp.is_expired(&env.block) => {
            cw721::Approval {
                spender: spender_addr,
                expires: exp,
            }
        }
        _ => cw721::Approval {
            spender: spender_addr,
            expires: Expiration::Never {},
        },
    };

    Ok(cw721::msg::ApprovalResponse { approval })
}

fn query_approvals(
    deps: Deps,
    env: Env,
    token_id: String,
    include_expired: Option<bool>,
) -> StdResult<cw721::msg::ApprovalsResponse> {
    // Verify token exists
    let _ = TOKENS.load(deps.storage, &token_id)?;
    let include_expired = include_expired.unwrap_or(false);

    // Get approvals from separate storage
    let approvals: Vec<cw721::Approval> = TOKEN_APPROVALS
        .prefix(&token_id)
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|r| r.ok())
        .filter(|(_, exp)| include_expired || !exp.is_expired(&env.block))
        .map(|(spender, exp)| cw721::Approval { spender, expires: exp })
        .collect();

    Ok(cw721::msg::ApprovalsResponse { approvals })
}
