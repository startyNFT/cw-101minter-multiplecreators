use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, IsAllowedResponse, QueryMsg, RoyaltyInfoResponse,
    TokenInfoResponse, TokensByCreatorResponse,
};
use crate::state::{increment_token_index, Config, TokenRoyalty, ALLOWLIST, CONFIG, TOKEN_ROYALTIES};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Decimal, Deps, DepsMut, Empty, Env, MessageInfo, Order, Reply,
    Response, StdResult, SubMsg, Timestamp, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
use cw_utils::parse_reply_instantiate_data;
use sg721::{CollectionInfo as Sg721CollectionInfo, ExecuteMsg as Sg721ExecuteMsg, InstantiateMsg as Sg721InstantiateMsg};
use sg721_royalty::msg::{TokenExtension, TokenRoyaltyInfo};
use url::Url;

const CONTRACT_NAME: &str = "crates.io:starty-multi-creator-minter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const INSTANTIATE_SG721_REPLY_ID: u64 = 1;

const DEFAULT_LIMIT: u32 = 30;
const MAX_LIMIT: u32 = 100;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate creator royalty (cannot exceed 10%)
    if msg.creator_royalty_bps > 1000 {
        return Err(ContractError::InvalidCreatorRoyalty {});
    }

    // Validate admin address
    let admin = info.sender.clone();

    let config = Config {
        admin: admin.clone(),
        collection_address: None, // Will be set in reply
        collection_code_id: msg.collection_params.code_id,
        creator_royalty_bps: msg.creator_royalty_bps,
        is_paused: false,
    };

    CONFIG.save(deps.storage, &config)?;

    // Add initial allowlist addresses
    for address_str in msg.initial_allowlist.iter() {
        let address = deps.api.addr_validate(address_str)?;
        ALLOWLIST.save(deps.storage, address, &true)?;
    }

    // Convert our CollectionInfo to Sg721CollectionInfo
    let collection_info = Sg721CollectionInfo {
        creator: msg.collection_params.info.creator,
        description: msg.collection_params.info.description,
        image: msg.collection_params.info.image,
        external_link: msg.collection_params.info.external_link,
        explicit_content: Some(false),
        start_trading_time: msg.collection_params.info.start_trading_time,
        royalty_info: None, // We handle royalties per-token
    };

    // Create SG721 collection instantiation message
    let wasm_msg = WasmMsg::Instantiate {
        code_id: msg.collection_params.code_id,
        msg: to_json_binary(&Sg721InstantiateMsg {
            name: msg.collection_params.name.clone(),
            symbol: msg.collection_params.symbol,
            minter: env.contract.address.to_string(),
            collection_info,
        })?,
        funds: vec![],
        admin: Some(admin.to_string()),
        label: format!(
            "SG721-{}-{}",
            msg.collection_params.code_id,
            msg.collection_params.name.trim()
        ),
    };

    let submsg = SubMsg::reply_on_success(wasm_msg, INSTANTIATE_SG721_REPLY_ID);

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("contract_name", CONTRACT_NAME)
        .add_attribute("contract_version", CONTRACT_VERSION)
        .add_attribute("admin", admin)
        .add_submessage(submsg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Mint { token_uri } => execute_mint(deps, env, info, token_uri),
        ExecuteMsg::AddToAllowlist { addresses } => execute_add_to_allowlist(deps, info, addresses),
        ExecuteMsg::RemoveFromAllowlist { addresses } => {
            execute_remove_from_allowlist(deps, info, addresses)
        }
        ExecuteMsg::SetPaused { paused } => execute_set_paused(deps, info, paused),
        ExecuteMsg::UpdateCreatorRoyalty { creator_royalty_bps } => {
            execute_update_creator_royalty(deps, info, creator_royalty_bps)
        }
        ExecuteMsg::UpdateStartTradingTime { start_time } => {
            execute_update_start_trading_time(deps, env, info, start_time)
        }
    }
}

pub fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_uri: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Check if minting is paused
    if config.is_paused {
        return Err(ContractError::MintingPaused {});
    }

    // Check if sender is on allowlist
    let is_allowed = ALLOWLIST.may_load(deps.storage, info.sender.clone())?.unwrap_or(false);
    if !is_allowed {
        return Err(ContractError::NotOnAllowlist {});
    }

    // Validate token URI is not empty
    if token_uri.is_empty() {
        return Err(ContractError::EmptyTokenUri {});
    }

    // Validate token URI format and scheme
    let parsed_url = Url::parse(&token_uri).map_err(|_| ContractError::InvalidTokenUri {
        uri: token_uri.clone(),
    })?;

    let scheme = parsed_url.scheme();
    if scheme != "https" && scheme != "ipfs" {
        return Err(ContractError::InvalidUriScheme {
            scheme: scheme.to_string(),
        });
    }

    // Get collection address
    let collection_address = config
        .collection_address
        .ok_or_else(|| ContractError::InstantiateSg721Error {})?;

    // Increment token index
    let token_id = increment_token_index(deps.storage)?.to_string();

    // Store token metadata (creator = sender)
    let token_royalty = TokenRoyalty {
        creator: info.sender.clone(),
        token_uri: token_uri.clone(),
        minted_at: env.block.time,
    };
    TOKEN_ROYALTIES.save(deps.storage, token_id.clone(), &token_royalty)?;

    // Create token extension with royalty info for the creator
    let extension = TokenExtension {
        royalty: Some(TokenRoyaltyInfo {
            payment_address: info.sender.to_string(),
            share: config.creator_royalty_bps,
        }),
    };

    // Create mint message for SG721 collection with royalty extension
    let mint_msg = Sg721ExecuteMsg::<TokenExtension, Empty>::Mint {
        token_id: token_id.clone(),
        owner: info.sender.to_string(),
        token_uri: Some(token_uri.clone()),
        extension,
    };

    let wasm_msg = WasmMsg::Execute {
        contract_addr: collection_address.to_string(),
        msg: to_json_binary(&mint_msg)?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "mint")
        .add_attribute("token_id", token_id)
        .add_attribute("creator", info.sender.to_string())
        .add_attribute("token_uri", token_uri))
}

pub fn execute_add_to_allowlist(
    deps: DepsMut,
    info: MessageInfo,
    addresses: Vec<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only admin can add to allowlist
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized(
            "Only admin can add to allowlist".to_string(),
        ));
    }

    // Add each address to allowlist
    for address_str in addresses.iter() {
        let address = deps.api.addr_validate(address_str)?;
        ALLOWLIST.save(deps.storage, address, &true)?;
    }

    Ok(Response::new()
        .add_attribute("action", "add_to_allowlist")
        .add_attribute("admin", info.sender)
        .add_attribute("count", addresses.len().to_string()))
}

pub fn execute_remove_from_allowlist(
    deps: DepsMut,
    info: MessageInfo,
    addresses: Vec<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only admin can remove from allowlist
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized(
            "Only admin can remove from allowlist".to_string(),
        ));
    }

    // Remove each address from allowlist
    for address_str in addresses.iter() {
        let address = deps.api.addr_validate(address_str)?;
        ALLOWLIST.remove(deps.storage, address);
    }

    Ok(Response::new()
        .add_attribute("action", "remove_from_allowlist")
        .add_attribute("admin", info.sender)
        .add_attribute("count", addresses.len().to_string()))
}

pub fn execute_set_paused(
    deps: DepsMut,
    info: MessageInfo,
    paused: bool,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // Only admin can pause/unpause
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized(
            "Only admin can pause/unpause".to_string(),
        ));
    }

    config.is_paused = paused;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "set_paused")
        .add_attribute("paused", paused.to_string())
        .add_attribute("admin", info.sender))
}

pub fn execute_update_creator_royalty(
    deps: DepsMut,
    info: MessageInfo,
    creator_royalty_bps: u64,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // Only admin can update creator royalty
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized(
            "Only admin can update creator royalty".to_string(),
        ));
    }

    // Validate creator royalty (cannot exceed 10%)
    if creator_royalty_bps > 1000 {
        return Err(ContractError::InvalidCreatorRoyalty {});
    }

    config.creator_royalty_bps = creator_royalty_bps;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_creator_royalty")
        .add_attribute("creator_royalty_bps", creator_royalty_bps.to_string())
        .add_attribute("admin", info.sender))
}

pub fn execute_update_start_trading_time(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    start_time: Option<Timestamp>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only admin can update trading time
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized(
            "Only admin can update trading time".to_string(),
        ));
    }

    // Validate start time is not in the past
    if let Some(time) = start_time {
        if time < env.block.time {
            return Err(ContractError::TradingTimeInPast {});
        }
    }

    let collection_address = config
        .collection_address
        .ok_or_else(|| ContractError::InstantiateSg721Error {})?;

    // Execute sg721 contract
    let msg = WasmMsg::Execute {
        contract_addr: collection_address.to_string(),
        msg: to_json_binary(&Sg721ExecuteMsg::<Empty, Empty>::UpdateStartTradingTime(
            start_time,
        ))?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_attribute("action", "update_start_trading_time")
        .add_attribute("admin", info.sender)
        .add_message(msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::RoyaltyInfo {
            token_id,
            sale_price,
        } => to_json_binary(&query_royalty_info(deps, token_id, sale_price)?),
        QueryMsg::TokenInfo { token_id } => to_json_binary(&query_token_info(deps, token_id)?),
        QueryMsg::TokensByCreator {
            creator,
            start_after,
            limit,
        } => to_json_binary(&query_tokens_by_creator(
            deps,
            creator,
            start_after,
            limit,
        )?),
        QueryMsg::IsAllowed { address } => to_json_binary(&query_is_allowed(deps, address)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

fn query_royalty_info(
    deps: Deps,
    token_id: String,
    sale_price: Uint128,
) -> StdResult<RoyaltyInfoResponse> {
    let config = CONFIG.load(deps.storage)?;
    let royalty = TOKEN_ROYALTIES
        .load(deps.storage, token_id.clone())
        .map_err(|_| {
            cosmwasm_std::StdError::generic_err(format!("Token not found: {}", token_id))
        })?;

    // Calculate royalty amount using global creator_royalty_bps with overflow protection
    let royalty_decimal = Decimal::bps(config.creator_royalty_bps);
    let royalty_amount = sale_price
        .checked_mul_floor(royalty_decimal)
        .map_err(|_| cosmwasm_std::StdError::generic_err("Calculation overflow"))?;

    Ok(RoyaltyInfoResponse {
        address: royalty.creator.to_string(),
        royalty_amount,
    })
}

fn query_token_info(deps: Deps, token_id: String) -> StdResult<TokenInfoResponse> {
    let token_royalty = TOKEN_ROYALTIES
        .load(deps.storage, token_id.clone())
        .map_err(|_| {
            cosmwasm_std::StdError::generic_err(format!("Token not found: {}", token_id))
        })?;

    Ok(TokenInfoResponse { token_royalty })
}

fn query_tokens_by_creator(
    deps: Deps,
    creator: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TokensByCreatorResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let creator_addr = deps.api.addr_validate(&creator)?;

    let tokens: Vec<String> = TOKEN_ROYALTIES
        .range(deps.storage, start, None, Order::Ascending)
        .filter_map(|item| {
            if let Ok((token_id, royalty)) = item {
                if royalty.creator == creator_addr {
                    Some(token_id)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .take(limit)
        .collect();

    Ok(TokensByCreatorResponse { tokens })
}

fn query_is_allowed(deps: Deps, address: String) -> StdResult<IsAllowedResponse> {
    let addr = deps.api.addr_validate(&address)?;
    let allowed = ALLOWLIST.may_load(deps.storage, addr)?.unwrap_or(false);
    Ok(IsAllowedResponse { allowed })
}

// Reply callback triggered from sg721 contract instantiation in instantiate()
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.id != INSTANTIATE_SG721_REPLY_ID {
        return Err(ContractError::InvalidReplyID {});
    }

    let reply = parse_reply_instantiate_data(msg);
    match reply {
        Ok(res) => {
            let collection_address = Addr::unchecked(res.contract_address.clone());

            // Update config with collection address
            let mut config = CONFIG.load(deps.storage)?;
            config.collection_address = Some(collection_address.clone());
            CONFIG.save(deps.storage, &config)?;

            Ok(Response::default()
                .add_attribute("action", "instantiate_sg721_reply")
                .add_attribute("sg721_address", collection_address))
        }
        Err(_) => Err(ContractError::InstantiateSg721Error {}),
    }
}
