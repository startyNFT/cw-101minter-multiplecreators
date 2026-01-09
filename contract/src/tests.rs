#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::error::ContractError;
    use crate::msg::{
        CollectionInfo, CollectionParams, ConfigResponse, ExecuteMsg, InstantiateMsg, IsAllowedResponse,
        QueryMsg, RoyaltyInfoResponse, TokenInfoResponse,
    };
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{from_json, Addr, OwnedDeps, Response, Uint128};

    const ADMIN: &str = "admin";
    const USER1: &str = "user1";
    const USER2: &str = "user2";
    const USER3: &str = "user3";
    const COLLECTION_CODE_ID: u64 = 2;

    // Helper function to create default instantiate message
    fn default_instantiate_msg() -> InstantiateMsg {
        InstantiateMsg {
            collection_params: CollectionParams {
                code_id: COLLECTION_CODE_ID,
                name: "Test Collection".to_string(),
                symbol: "TEST".to_string(),
                info: CollectionInfo {
                    creator: ADMIN.to_string(),
                    description: "Test collection for unit tests".to_string(),
                    image: "ipfs://QmTest123".to_string(),
                    external_link: Some("https://test.com".to_string()),
                    start_trading_time: None,
                },
            },
            creator_royalty_bps: 500, // 5% global royalty
            initial_allowlist: vec![USER1.to_string(), USER2.to_string()],
        }
    }

    // Helper function to setup contract
    fn setup_contract(
        deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ) -> Result<Response, ContractError> {
        let msg = default_instantiate_msg();
        let info = mock_info(ADMIN, &[]);
        let env = mock_env();
        instantiate(deps.as_mut(), env, info, msg)
    }

    // Helper function to set collection address directly (simulating successful SG721 creation)
    fn set_collection_address(
        deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
        collection_addr: &str,
    ) {
        use crate::state::CONFIG;
        let mut config = CONFIG.load(deps.as_ref().storage).unwrap();
        config.collection_address = Some(Addr::unchecked(collection_addr));
        CONFIG.save(deps.as_mut().storage, &config).unwrap();
    }

    #[test]
    fn test_instantiate_success() {
        let mut deps = mock_dependencies();
        let msg = default_instantiate_msg();
        let info = mock_info(ADMIN, &[]);
        let env = mock_env();

        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        // Should have 1 submessage (SG721 instantiation)
        assert_eq!(res.messages.len(), 1);

        // Check config was saved
        let config_query = QueryMsg::Config {};
        let res: ConfigResponse =
            from_json(query(deps.as_ref(), mock_env(), config_query).unwrap()).unwrap();

        assert_eq!(res.config.admin, Addr::unchecked(ADMIN));
        assert_eq!(res.config.collection_address, None); // Not set yet
        assert_eq!(res.config.creator_royalty_bps, 500); // 5%
        assert_eq!(res.config.is_paused, false);

        // Check allowlist was populated
        let is_allowed_query = QueryMsg::IsAllowed {
            address: USER1.to_string(),
        };
        let res: IsAllowedResponse =
            from_json(query(deps.as_ref(), mock_env(), is_allowed_query).unwrap()).unwrap();
        assert_eq!(res.allowed, true);

        let is_allowed_query = QueryMsg::IsAllowed {
            address: USER3.to_string(),
        };
        let res: IsAllowedResponse =
            from_json(query(deps.as_ref(), mock_env(), is_allowed_query).unwrap()).unwrap();
        assert_eq!(res.allowed, false);
    }

    #[test]
    fn test_instantiate_invalid_creator_royalty() {
        let mut deps = mock_dependencies();
        let mut msg = default_instantiate_msg();
        msg.creator_royalty_bps = 1001; // Over 10%

        let info = mock_info(ADMIN, &[]);
        let env = mock_env();

        let err = instantiate(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidCreatorRoyalty {});
    }

    #[test]
    fn test_mint_success() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();
        set_collection_address(&mut deps, "collection123");

        // USER1 is on allowlist, should be able to mint
        let mint_msg = ExecuteMsg::Mint {
            token_uri: "ipfs://QmToken1".to_string(),
        };

        let info = mock_info(USER1, &[]);
        let env = mock_env();

        let res = execute(deps.as_mut(), env, info, mint_msg).unwrap();

        // Should have 1 message (SG721 mint)
        assert_eq!(res.messages.len(), 1);

        // Check attributes
        assert_eq!(res.attributes[0].key, "action");
        assert_eq!(res.attributes[0].value, "mint");
        assert_eq!(res.attributes[1].key, "token_id");
        assert_eq!(res.attributes[1].value, "1"); // First token
        assert_eq!(res.attributes[2].key, "creator");
        assert_eq!(res.attributes[2].value, USER1);

        // Check token royalty was stored
        let token_info_query = QueryMsg::TokenInfo {
            token_id: "1".to_string(),
        };
        let res: TokenInfoResponse =
            from_json(query(deps.as_ref(), mock_env(), token_info_query).unwrap()).unwrap();

        assert_eq!(res.token_royalty.creator, Addr::unchecked(USER1));
        assert_eq!(res.token_royalty.token_uri, "ipfs://QmToken1");
    }

    #[test]
    fn test_mint_not_on_allowlist() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();
        set_collection_address(&mut deps, "collection123");

        // USER3 is NOT on allowlist
        let mint_msg = ExecuteMsg::Mint {
            token_uri: "ipfs://QmToken1".to_string(),
        };

        let info = mock_info(USER3, &[]);
        let env = mock_env();

        let err = execute(deps.as_mut(), env, info, mint_msg).unwrap_err();
        assert_eq!(err, ContractError::NotOnAllowlist {});
    }

    #[test]
    fn test_mint_while_paused() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();
        set_collection_address(&mut deps, "collection123");

        // Pause minting
        let pause_msg = ExecuteMsg::SetPaused { paused: true };
        let info = mock_info(ADMIN, &[]);
        execute(deps.as_mut(), mock_env(), info, pause_msg).unwrap();

        // Try to mint
        let mint_msg = ExecuteMsg::Mint {
            token_uri: "ipfs://QmToken1".to_string(),
        };

        let info = mock_info(USER1, &[]);
        let env = mock_env();

        let err = execute(deps.as_mut(), env, info, mint_msg).unwrap_err();
        assert_eq!(err, ContractError::MintingPaused {});
    }

    #[test]
    fn test_mint_empty_token_uri() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();
        set_collection_address(&mut deps, "collection123");

        // Try to mint with empty URI
        let mint_msg = ExecuteMsg::Mint {
            token_uri: "".to_string(),
        };

        let info = mock_info(USER1, &[]);
        let env = mock_env();

        let err = execute(deps.as_mut(), env, info, mint_msg).unwrap_err();
        assert_eq!(err, ContractError::EmptyTokenUri {});
    }

    #[test]
    fn test_mint_invalid_token_uri() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();
        set_collection_address(&mut deps, "collection123");

        // Try to mint with invalid URI
        let mint_msg = ExecuteMsg::Mint {
            token_uri: "not-a-url".to_string(),
        };

        let info = mock_info(USER1, &[]);
        let env = mock_env();

        let err = execute(deps.as_mut(), env, info, mint_msg).unwrap_err();
        assert!(matches!(err, ContractError::InvalidTokenUri { .. }));
    }

    #[test]
    fn test_mint_invalid_uri_scheme() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();
        set_collection_address(&mut deps, "collection123");

        // Try to mint with http (not https)
        let mint_msg = ExecuteMsg::Mint {
            token_uri: "http://example.com/token.json".to_string(),
        };

        let info = mock_info(USER1, &[]);
        let env = mock_env();

        let err = execute(deps.as_mut(), env, info, mint_msg).unwrap_err();
        assert!(matches!(err, ContractError::InvalidUriScheme { .. }));
    }

    #[test]
    fn test_mint_sequential_token_ids() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();
        set_collection_address(&mut deps, "collection123");

        // Mint 3 tokens
        for i in 1..=3 {
            let mint_msg = ExecuteMsg::Mint {
                token_uri: format!("ipfs://QmToken{}", i),
            };

            let info = mock_info(USER1, &[]);
            let env = mock_env();

            let res = execute(deps.as_mut(), env, info, mint_msg).unwrap();
            assert_eq!(res.attributes[1].value, i.to_string());
        }
    }

    #[test]
    fn test_add_to_allowlist_admin_only() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        // Non-admin tries to add to allowlist
        let add_msg = ExecuteMsg::AddToAllowlist {
            addresses: vec![USER3.to_string()],
        };

        let info = mock_info(USER1, &[]);
        let env = mock_env();

        let err = execute(deps.as_mut(), env, info, add_msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized(_)));
    }

    #[test]
    fn test_add_to_allowlist_success() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        // Check USER3 is not allowed initially
        let is_allowed_query = QueryMsg::IsAllowed {
            address: USER3.to_string(),
        };
        let res: IsAllowedResponse =
            from_json(query(deps.as_ref(), mock_env(), is_allowed_query).unwrap()).unwrap();
        assert_eq!(res.allowed, false);

        // Admin adds USER3 to allowlist
        let add_msg = ExecuteMsg::AddToAllowlist {
            addresses: vec![USER3.to_string()],
        };

        let info = mock_info(ADMIN, &[]);
        let env = mock_env();

        let res = execute(deps.as_mut(), env, info, add_msg).unwrap();
        assert_eq!(res.attributes[0].value, "add_to_allowlist");
        assert_eq!(res.attributes[2].value, "1"); // count

        // Check USER3 is now allowed
        let is_allowed_query = QueryMsg::IsAllowed {
            address: USER3.to_string(),
        };
        let res: IsAllowedResponse =
            from_json(query(deps.as_ref(), mock_env(), is_allowed_query).unwrap()).unwrap();
        assert_eq!(res.allowed, true);
    }

    #[test]
    fn test_remove_from_allowlist_admin_only() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        // Non-admin tries to remove from allowlist
        let remove_msg = ExecuteMsg::RemoveFromAllowlist {
            addresses: vec![USER1.to_string()],
        };

        let info = mock_info(USER1, &[]);
        let env = mock_env();

        let err = execute(deps.as_mut(), env, info, remove_msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized(_)));
    }

    #[test]
    fn test_remove_from_allowlist_success() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        // Check USER1 is allowed initially
        let is_allowed_query = QueryMsg::IsAllowed {
            address: USER1.to_string(),
        };
        let res: IsAllowedResponse =
            from_json(query(deps.as_ref(), mock_env(), is_allowed_query).unwrap()).unwrap();
        assert_eq!(res.allowed, true);

        // Admin removes USER1 from allowlist
        let remove_msg = ExecuteMsg::RemoveFromAllowlist {
            addresses: vec![USER1.to_string()],
        };

        let info = mock_info(ADMIN, &[]);
        let env = mock_env();

        let res = execute(deps.as_mut(), env, info, remove_msg).unwrap();
        assert_eq!(res.attributes[0].value, "remove_from_allowlist");
        assert_eq!(res.attributes[2].value, "1"); // count

        // Check USER1 is no longer allowed
        let is_allowed_query = QueryMsg::IsAllowed {
            address: USER1.to_string(),
        };
        let res: IsAllowedResponse =
            from_json(query(deps.as_ref(), mock_env(), is_allowed_query).unwrap()).unwrap();
        assert_eq!(res.allowed, false);
    }

    #[test]
    fn test_set_paused_admin_only() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        // Non-admin tries to pause
        let pause_msg = ExecuteMsg::SetPaused { paused: true };

        let info = mock_info(USER1, &[]);
        let env = mock_env();

        let err = execute(deps.as_mut(), env, info, pause_msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized(_)));
    }

    #[test]
    fn test_set_paused_success() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        // Admin pauses
        let pause_msg = ExecuteMsg::SetPaused { paused: true };

        let info = mock_info(ADMIN, &[]);
        let env = mock_env();

        let res = execute(deps.as_mut(), env, info, pause_msg).unwrap();
        assert_eq!(res.attributes[0].value, "set_paused");
        assert_eq!(res.attributes[1].value, "true");

        // Check config
        let config_query = QueryMsg::Config {};
        let res: ConfigResponse =
            from_json(query(deps.as_ref(), mock_env(), config_query).unwrap()).unwrap();
        assert_eq!(res.config.is_paused, true);
    }

    #[test]
    fn test_update_creator_royalty_admin_only() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        // Non-admin tries to update
        let update_msg = ExecuteMsg::UpdateCreatorRoyalty {
            creator_royalty_bps: 1000,
        };

        let info = mock_info(USER1, &[]);
        let env = mock_env();

        let err = execute(deps.as_mut(), env, info, update_msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized(_)));
    }

    #[test]
    fn test_update_creator_royalty_too_high() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        // Admin tries to set > 10%
        let update_msg = ExecuteMsg::UpdateCreatorRoyalty {
            creator_royalty_bps: 1001,
        };

        let info = mock_info(ADMIN, &[]);
        let env = mock_env();

        let err = execute(deps.as_mut(), env, info, update_msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidCreatorRoyalty {});
    }

    #[test]
    fn test_update_creator_royalty_success() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        // Admin updates to 10%
        let update_msg = ExecuteMsg::UpdateCreatorRoyalty {
            creator_royalty_bps: 1000,
        };

        let info = mock_info(ADMIN, &[]);
        let env = mock_env();

        let res = execute(deps.as_mut(), env, info, update_msg).unwrap();
        assert_eq!(res.attributes[0].value, "update_creator_royalty");
        assert_eq!(res.attributes[1].value, "1000");

        // Check config
        let config_query = QueryMsg::Config {};
        let res: ConfigResponse =
            from_json(query(deps.as_ref(), mock_env(), config_query).unwrap()).unwrap();
        assert_eq!(res.config.creator_royalty_bps, 1000);
    }

    #[test]
    fn test_query_royalty_info() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();
        set_collection_address(&mut deps, "collection123");

        // USER1 mints
        let mint_msg = ExecuteMsg::Mint {
            token_uri: "ipfs://QmToken1".to_string(),
        };
        execute(deps.as_mut(), mock_env(), mock_info(USER1, &[]), mint_msg).unwrap();

        // Query royalty for sale price of 1000000 (with 5% royalty)
        let query_msg = QueryMsg::RoyaltyInfo {
            token_id: "1".to_string(),
            sale_price: Uint128::new(1000000),
        };

        let res: RoyaltyInfoResponse =
            from_json(query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();

        assert_eq!(res.address, USER1);
        assert_eq!(res.royalty_amount, Uint128::new(50000)); // 5% of 1000000
    }

    #[test]
    fn test_query_royalty_info_uses_global_percentage() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();
        set_collection_address(&mut deps, "collection123");

        // USER1 mints
        let mint_msg = ExecuteMsg::Mint {
            token_uri: "ipfs://QmToken1".to_string(),
        };
        execute(deps.as_mut(), mock_env(), mock_info(USER1, &[]), mint_msg).unwrap();

        // Admin updates global royalty to 10%
        let update_msg = ExecuteMsg::UpdateCreatorRoyalty {
            creator_royalty_bps: 1000, // 10%
        };
        execute(deps.as_mut(), mock_env(), mock_info(ADMIN, &[]), update_msg).unwrap();

        // Query royalty - should use NEW global percentage
        let query_msg = QueryMsg::RoyaltyInfo {
            token_id: "1".to_string(),
            sale_price: Uint128::new(1000000),
        };

        let res: RoyaltyInfoResponse =
            from_json(query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();

        assert_eq!(res.address, USER1);
        assert_eq!(res.royalty_amount, Uint128::new(100000)); // 10% of 1000000 (NOT 5%)
    }

    #[test]
    fn test_query_royalty_info_not_found() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();

        // Query non-existent token
        let query_msg = QueryMsg::RoyaltyInfo {
            token_id: "999".to_string(),
            sale_price: Uint128::new(1000000),
        };

        let err = query(deps.as_ref(), mock_env(), query_msg).unwrap_err();
        assert!(err.to_string().contains("Token not found"));
    }

    #[test]
    fn test_multiple_creators_same_collection() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();
        set_collection_address(&mut deps, "collection123");

        // USER1 mints
        let mint_msg = ExecuteMsg::Mint {
            token_uri: "ipfs://QmToken1".to_string(),
        };
        execute(deps.as_mut(), mock_env(), mock_info(USER1, &[]), mint_msg).unwrap();

        // USER2 mints
        let mint_msg = ExecuteMsg::Mint {
            token_uri: "ipfs://QmToken2".to_string(),
        };
        execute(deps.as_mut(), mock_env(), mock_info(USER2, &[]), mint_msg).unwrap();

        // Check token 1 royalty (USER1)
        let query_msg = QueryMsg::RoyaltyInfo {
            token_id: "1".to_string(),
            sale_price: Uint128::new(1000000),
        };
        let res: RoyaltyInfoResponse =
            from_json(query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();
        assert_eq!(res.address, USER1);
        assert_eq!(res.royalty_amount, Uint128::new(50000)); // 5%

        // Check token 2 royalty (USER2)
        let query_msg = QueryMsg::RoyaltyInfo {
            token_id: "2".to_string(),
            sale_price: Uint128::new(1000000),
        };
        let res: RoyaltyInfoResponse =
            from_json(query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();
        assert_eq!(res.address, USER2);
        assert_eq!(res.royalty_amount, Uint128::new(50000)); // 5% (same global %)
    }

    #[test]
    fn test_https_uri_allowed() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();
        set_collection_address(&mut deps, "collection123");

        // Mint with HTTPS URI
        let mint_msg = ExecuteMsg::Mint {
            token_uri: "https://example.com/token.json".to_string(),
        };

        let res = execute(deps.as_mut(), mock_env(), mock_info(USER1, &[]), mint_msg).unwrap();
        assert_eq!(res.attributes[0].value, "mint");
    }

    #[test]
    fn test_ipfs_uri_allowed() {
        let mut deps = mock_dependencies();
        setup_contract(&mut deps).unwrap();
        set_collection_address(&mut deps, "collection123");

        // Mint with IPFS URI
        let mint_msg = ExecuteMsg::Mint {
            token_uri: "ipfs://QmTest123".to_string(),
        };

        let res = execute(deps.as_mut(), mock_env(), mock_info(USER1, &[]), mint_msg).unwrap();
        assert_eq!(res.attributes[0].value, "mint");
    }
}
