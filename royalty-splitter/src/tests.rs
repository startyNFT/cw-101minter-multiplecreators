#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query, sudo};
    use crate::error::ContractError;
    use crate::msg::{
        ConfigResponse, CreatorStatsResponse, ExecuteMsg, InstantiateMsg, PendingBalanceResponse,
        QueryMsg, SudoMsg, TokenRoyaltiesResponse,
    };
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{from_json, Addr, Coin, OwnedDeps, Response, Uint128};

    const ADMIN: &str = "admin";
    const MINTER: &str = "minter_contract";
    const COLLECTION: &str = "collection_contract";
    const CREATOR1: &str = "creator1";

    fn default_instantiate_msg() -> InstantiateMsg {
        InstantiateMsg {
            minter: MINTER.to_string(),
            collection: COLLECTION.to_string(),
            royalty_bps: 500, // 5%
        }
    }

    fn setup_contract(
        deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ) -> Result<Response, ContractError> {
        let msg = default_instantiate_msg();
        let info = mock_info(ADMIN, &[]);
        let env = mock_env();
        instantiate(deps.as_mut(), env, info, msg)
    }

    #[test]
    fn test_instantiate_success() {
        let mut deps = mock_dependencies();
        let msg = default_instantiate_msg();
        let info = mock_info(ADMIN, &[]);
        let env = mock_env();

        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(res.attributes.len(), 5);
        assert_eq!(res.attributes[0].key, "action");
        assert_eq!(res.attributes[0].value, "instantiate");

        // Check config
        let config_query = QueryMsg::Config {};
        let res: ConfigResponse =
            from_json(query(deps.as_ref(), mock_env(), config_query).unwrap()).unwrap();

        assert_eq!(res.config.admin, Addr::unchecked(ADMIN));
        assert_eq!(res.config.minter, Addr::unchecked(MINTER));
        assert_eq!(res.config.collection, Addr::unchecked(COLLECTION));
        assert_eq!(res.config.royalty_bps, 500);
    }

    #[test]
    fn test_instantiate_invalid_royalty_bps() {
        let mut deps = mock_dependencies();
        let mut msg = default_instantiate_msg();
        msg.royalty_bps = 10001; // Over 100%

        let info = mock_info(ADMIN, &[]);
        let env = mock_env();

        let err = instantiate(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidRoyaltyBps { bps: 10001 });
    }

    #[test]
    fn test_update_minter_admin_only() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        // Non-admin tries to update
        let update_msg = ExecuteMsg::UpdateMinter {
            minter: "new_minter".to_string(),
        };

        let info = mock_info("not_admin", &[]);
        let env = mock_env();

        let err = execute(deps.as_mut(), env, info, update_msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized(_)));
    }

    #[test]
    fn test_update_minter_success() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        let update_msg = ExecuteMsg::UpdateMinter {
            minter: "new_minter".to_string(),
        };

        let info = mock_info(ADMIN, &[]);
        let env = mock_env();

        let res = execute(deps.as_mut(), env, info, update_msg).unwrap();
        assert_eq!(res.attributes[0].value, "update_minter");

        // Verify config updated
        let config_query = QueryMsg::Config {};
        let res: ConfigResponse =
            from_json(query(deps.as_ref(), mock_env(), config_query).unwrap()).unwrap();
        assert_eq!(res.config.minter, Addr::unchecked("new_minter"));
    }

    #[test]
    fn test_update_collection_success() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        let update_msg = ExecuteMsg::UpdateCollection {
            collection: "new_collection".to_string(),
        };

        let info = mock_info(ADMIN, &[]);
        let env = mock_env();

        let res = execute(deps.as_mut(), env, info, update_msg).unwrap();
        assert_eq!(res.attributes[0].value, "update_collection");

        // Verify config updated
        let config_query = QueryMsg::Config {};
        let res: ConfigResponse =
            from_json(query(deps.as_ref(), mock_env(), config_query).unwrap()).unwrap();
        assert_eq!(res.config.collection, Addr::unchecked("new_collection"));
    }

    #[test]
    fn test_update_royalty_bps_success() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        let update_msg = ExecuteMsg::UpdateRoyaltyBps { royalty_bps: 1000 };

        let info = mock_info(ADMIN, &[]);
        let env = mock_env();

        let res = execute(deps.as_mut(), env, info, update_msg).unwrap();
        assert_eq!(res.attributes[0].value, "update_royalty_bps");

        // Verify config updated
        let config_query = QueryMsg::Config {};
        let res: ConfigResponse =
            from_json(query(deps.as_ref(), mock_env(), config_query).unwrap()).unwrap();
        assert_eq!(res.config.royalty_bps, 1000);
    }

    #[test]
    fn test_update_royalty_bps_too_high() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        let update_msg = ExecuteMsg::UpdateRoyaltyBps { royalty_bps: 10001 };

        let info = mock_info(ADMIN, &[]);
        let env = mock_env();

        let err = execute(deps.as_mut(), env, info, update_msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidRoyaltyBps { bps: 10001 });
    }

    #[test]
    fn test_sale_hook_different_collection_ignored() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        // Sale hook from different collection
        let sudo_msg = SudoMsg::SaleHook {
            collection: "other_collection".to_string(),
            token_id: 1,
            price: Coin {
                denom: "ustars".to_string(),
                amount: Uint128::new(1000000),
            },
            seller: "seller".to_string(),
            buyer: "buyer".to_string(),
        };

        let env = mock_env();
        let res = sudo(deps.as_mut(), env, sudo_msg).unwrap();

        // Should be ignored
        assert_eq!(res.attributes[0].value, "sale_hook_ignored");
        assert_eq!(res.attributes[1].value, "different_collection");
    }

    #[test]
    fn test_query_pending_balance() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        let query_msg = QueryMsg::PendingBalance {};
        let res: PendingBalanceResponse =
            from_json(query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();

        assert_eq!(res.pending, Uint128::zero());
    }

    #[test]
    fn test_query_creator_stats_empty() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        let query_msg = QueryMsg::CreatorStats {
            creator: CREATOR1.to_string(),
        };
        let res: CreatorStatsResponse =
            from_json(query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();

        assert_eq!(res.creator, CREATOR1);
        assert_eq!(res.stats.total_received, Uint128::zero());
        assert_eq!(res.stats.total_sales, 0);
    }

    #[test]
    fn test_query_token_royalties_empty() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        let query_msg = QueryMsg::TokenRoyalties { token_id: 1 };
        let res: TokenRoyaltiesResponse =
            from_json(query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();

        assert_eq!(res.token_id, 1);
        assert_eq!(res.total_royalties, Uint128::zero());
    }

    #[test]
    fn test_emergency_withdraw_admin_only() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        let withdraw_msg = ExecuteMsg::EmergencyWithdraw {
            recipient: "recipient".to_string(),
            amount: Uint128::new(1000),
        };

        let info = mock_info("not_admin", &[]);
        let env = mock_env();

        let err = execute(deps.as_mut(), env, info, withdraw_msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized(_)));
    }

    #[test]
    fn test_emergency_withdraw_success() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        let withdraw_msg = ExecuteMsg::EmergencyWithdraw {
            recipient: "recipient".to_string(),
            amount: Uint128::new(1000),
        };

        let info = mock_info(ADMIN, &[]);
        let env = mock_env();

        let res = execute(deps.as_mut(), env, info, withdraw_msg).unwrap();
        assert_eq!(res.attributes[0].value, "emergency_withdraw");
        assert_eq!(res.messages.len(), 1); // BankMsg::Send
    }

    #[test]
    fn test_manual_distribute_admin_only() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        let distribute_msg = ExecuteMsg::ManualDistribute {
            token_id: 1,
            amount: Uint128::new(50000),
        };

        let info = mock_info("not_admin", &[]);
        let env = mock_env();

        let err = execute(deps.as_mut(), env, info, distribute_msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized(_)));
    }
}
