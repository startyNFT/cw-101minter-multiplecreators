use crate::error::ContractError;
use crate::msg::{CheckRoyaltiesResponse, QueryMsg, RoyaltyInfoResponse, TokenExtension};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Decimal, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
    Uint128,
};
use cw2::set_contract_version;
use cw721::Cw721Query;
use sg721::InstantiateMsg;
use sg721_base::Sg721Contract;

const CONTRACT_NAME: &str = "crates.io:sg721-royalty";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Type alias for our custom SG721 with TokenExtension
pub type Sg721Royalty<'a> = Sg721Contract<'a, TokenExtension>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Use sg721-base instantiate
    let res = Sg721Royalty::default().instantiate(deps, env, info, msg)?;

    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: sg721::ExecuteMsg<TokenExtension, Empty>,
) -> Result<Response, ContractError> {
    // Delegate to sg721-base execute
    let res = Sg721Royalty::default().execute(deps, env, info, msg)?;
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // CW2981 queries handled directly
        QueryMsg::RoyaltyInfo {
            token_id,
            sale_price,
        } => to_json_binary(&query_royalty_info(deps, token_id, sale_price)?),
        QueryMsg::CheckRoyalties {} => to_json_binary(&query_check_royalties()),
        // All other queries forwarded to sg721-base
        _ => Sg721Royalty::default().query(deps, env, msg.into()),
    }
}

/// CW2981: Query royalty info for a specific token sale
fn query_royalty_info(
    deps: Deps,
    token_id: String,
    sale_price: Uint128,
) -> StdResult<RoyaltyInfoResponse> {
    let contract = Sg721Royalty::default();

    // Get the NFT info with extension using the cw721 trait method
    let nft_info = contract.parent.nft_info(deps, token_id)?;

    // Check if token has royalty info in its extension
    if let Some(ref royalty) = nft_info.extension.royalty {
        // Calculate royalty amount: sale_price * (share / 10000)
        let royalty_decimal = Decimal::bps(royalty.share);
        let royalty_amount = sale_price
            .checked_mul_floor(royalty_decimal)
            .map_err(|_| cosmwasm_std::StdError::generic_err("Calculation overflow"))?;

        return Ok(RoyaltyInfoResponse {
            address: royalty.payment_address.clone(),
            royalty_amount,
        });
    }

    // No royalty info found - return zero
    Ok(RoyaltyInfoResponse {
        address: String::new(),
        royalty_amount: Uint128::zero(),
    })
}

/// CW2981: Check if this contract supports royalty queries
fn query_check_royalties() -> CheckRoyaltiesResponse {
    CheckRoyaltiesResponse {
        royalty_payments: true,
    }
}
