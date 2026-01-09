use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, IsAllowedResponse, QueryMsg, TokenCountResponse};
use crate::state::{increment_token_index, Config, ALLOWLIST, CONFIG, TOKEN_INDEX};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
    SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::parse_instantiate_response_data;
use multi_creator_collection::msg::{
    CollectionExtension, ExecuteMsg as CollectionExecuteMsg, GeneralRoyaltyInfo,
    InstantiateMsg as CollectionInstantiateMsg, TokenExtension,
};
use url::Url;

const CONTRACT_NAME: &str = "crates.io:multi-creator-minter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const INSTANTIATE_COLLECTION_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate royalty (max 10%)
    if msg.royalty_bps > 1000 {
        return Err(ContractError::InvalidRoyalty {});
    }

    let admin = info.sender.clone();

    let config = Config {
        admin: admin.clone(),
        collection: None, // Set in reply
        collection_code_id: msg.collection_params.code_id,
        royalty_bps: msg.royalty_bps,
        is_paused: false,
    };

    CONFIG.save(deps.storage, &config)?;
    TOKEN_INDEX.save(deps.storage, &0u64)?;

    // Add initial allowlist
    for addr_str in &msg.initial_allowlist {
        let addr = deps.api.addr_validate(addr_str)?;
        ALLOWLIST.save(deps.storage, &addr, &true)?;
    }

    // Instantiate collection contract
    let collection_info = CollectionExtension {
        description: msg.collection_params.description,
        image: msg.collection_params.image,
        external_link: msg.collection_params.external_link,
        explicit_content: Some(false),
        start_trading_time: None,
    };

    // Set up general royalty with admin as default recipient
    let general_royalty = GeneralRoyaltyInfo {
        address: admin.to_string(),
        royalty_bps: msg.royalty_bps,
    };

    let collection_msg = CollectionInstantiateMsg {
        name: msg.collection_params.name.clone(),
        symbol: msg.collection_params.symbol.clone(),
        minter: env.contract.address.to_string(),
        creator: Some(admin.to_string()),
        collection_info: Some(collection_info),
        general_royalty: Some(general_royalty),
    };

    let wasm_msg = WasmMsg::Instantiate {
        code_id: msg.collection_params.code_id,
        msg: to_json_binary(&collection_msg)?,
        funds: vec![],
        admin: Some(admin.to_string()),
        label: format!("collection-{}-{}", msg.collection_params.code_id, msg.collection_params.name.trim()),
    };

    let submsg = SubMsg::reply_on_success(wasm_msg, INSTANTIATE_COLLECTION_REPLY_ID);

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", admin)
        .add_attribute("royalty_bps", msg.royalty_bps.to_string())
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
        ExecuteMsg::Mint {
            token_uri,
            use_per_token_royalty,
        } => execute_mint(deps, env, info, token_uri, use_per_token_royalty),
        ExecuteMsg::AddToAllowlist { addresses } => execute_add_to_allowlist(deps, info, addresses),
        ExecuteMsg::RemoveFromAllowlist { addresses } => {
            execute_remove_from_allowlist(deps, info, addresses)
        }
        ExecuteMsg::SetPaused { paused } => execute_set_paused(deps, info, paused),
        ExecuteMsg::UpdateRoyalty { royalty_bps } => execute_update_royalty(deps, info, royalty_bps),
        ExecuteMsg::UpdateAdmin { new_admin } => execute_update_admin(deps, info, new_admin),
    }
}

fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_uri: String,
    use_per_token_royalty: Option<bool>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Check paused
    if config.is_paused {
        return Err(ContractError::MintingPaused {});
    }

    // Check allowlist
    let is_allowed = ALLOWLIST.may_load(deps.storage, &info.sender)?.unwrap_or(false);
    if !is_allowed {
        return Err(ContractError::NotOnAllowlist {});
    }

    // Validate token URI
    if token_uri.is_empty() {
        return Err(ContractError::EmptyTokenUri {});
    }

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
    let collection = config
        .collection
        .ok_or(ContractError::CollectionNotInitialized {})?;

    // Generate token ID
    let token_id = increment_token_index(deps.storage)?.to_string();

    // Create token extension
    // If use_per_token_royalty is true (default), set per-token royalty with minter as creator
    // If false, leave creator/royalty_bps as None to use general royalty
    let use_per_token = use_per_token_royalty.unwrap_or(true);
    let extension = if use_per_token {
        TokenExtension {
            creator: Some(info.sender.to_string()),
            royalty_bps: Some(config.royalty_bps),
            minted_at: Some(env.block.time),
        }
    } else {
        TokenExtension {
            creator: None,
            royalty_bps: None,
            minted_at: Some(env.block.time),
        }
    };

    // Create mint message
    let mint_msg = CollectionExecuteMsg::Mint {
        token_id: token_id.clone(),
        owner: info.sender.to_string(),
        token_uri: Some(token_uri.clone()),
        extension,
    };

    let wasm_msg = WasmMsg::Execute {
        contract_addr: collection.to_string(),
        msg: to_json_binary(&mint_msg)?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "mint")
        .add_attribute("token_id", token_id)
        .add_attribute("creator", info.sender)
        .add_attribute("token_uri", token_uri)
        .add_attribute("per_token_royalty", use_per_token.to_string()))
}

fn execute_add_to_allowlist(
    deps: DepsMut,
    info: MessageInfo,
    addresses: Vec<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized(
            "Only admin can modify allowlist".to_string(),
        ));
    }

    for addr_str in &addresses {
        let addr = deps.api.addr_validate(addr_str)?;
        ALLOWLIST.save(deps.storage, &addr, &true)?;
    }

    Ok(Response::new()
        .add_attribute("action", "add_to_allowlist")
        .add_attribute("count", addresses.len().to_string()))
}

fn execute_remove_from_allowlist(
    deps: DepsMut,
    info: MessageInfo,
    addresses: Vec<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized(
            "Only admin can modify allowlist".to_string(),
        ));
    }

    for addr_str in &addresses {
        let addr = deps.api.addr_validate(addr_str)?;
        ALLOWLIST.remove(deps.storage, &addr);
    }

    Ok(Response::new()
        .add_attribute("action", "remove_from_allowlist")
        .add_attribute("count", addresses.len().to_string()))
}

fn execute_set_paused(
    deps: DepsMut,
    info: MessageInfo,
    paused: bool,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized(
            "Only admin can pause/unpause".to_string(),
        ));
    }

    config.is_paused = paused;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "set_paused")
        .add_attribute("paused", paused.to_string()))
}

fn execute_update_royalty(
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

    if royalty_bps > 1000 {
        return Err(ContractError::InvalidRoyalty {});
    }

    config.royalty_bps = royalty_bps;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_royalty")
        .add_attribute("royalty_bps", royalty_bps.to_string()))
}

fn execute_update_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized(
            "Only admin can transfer admin".to_string(),
        ));
    }

    let new_admin_addr = deps.api.addr_validate(&new_admin)?;
    config.admin = new_admin_addr.clone();
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_admin")
        .add_attribute("new_admin", new_admin_addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::IsAllowed { address } => to_json_binary(&query_is_allowed(deps, address)?),
        QueryMsg::TokenCount {} => to_json_binary(&query_token_count(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

fn query_is_allowed(deps: Deps, address: String) -> StdResult<IsAllowedResponse> {
    let addr = deps.api.addr_validate(&address)?;
    let allowed = ALLOWLIST.may_load(deps.storage, &addr)?.unwrap_or(false);
    Ok(IsAllowedResponse { allowed })
}

fn query_token_count(deps: Deps) -> StdResult<TokenCountResponse> {
    let count = TOKEN_INDEX.load(deps.storage)?;
    Ok(TokenCountResponse { count })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.id != INSTANTIATE_COLLECTION_REPLY_ID {
        return Err(ContractError::InvalidReplyId {});
    }

    let result = msg
        .result
        .into_result()
        .map_err(|_| ContractError::InstantiateCollectionError {})?;

    let data = result
        .msg_responses
        .first()
        .ok_or(ContractError::InstantiateCollectionError {})?
        .value
        .clone();

    let reply = parse_instantiate_response_data(&data)
        .map_err(|_| ContractError::InstantiateCollectionError {})?;

    let collection_addr = Addr::unchecked(&reply.contract_address);

    let mut config = CONFIG.load(deps.storage)?;
    config.collection = Some(collection_addr.clone());
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate_collection_reply")
        .add_attribute("collection", collection_addr))
}
