#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::{Addr, Empty};
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};

    use crate::msg::{
        CollectionParams, ConfigResponse, ExecuteMsg, InstantiateMsg, IsAllowedResponse, QueryMsg,
        TokenCountResponse,
    };

    fn mock_addr(name: &str) -> Addr {
        MockApi::default().addr_make(name)
    }

    fn admin_addr() -> Addr {
        mock_addr("admin")
    }

    fn minter1_addr() -> Addr {
        mock_addr("minter1")
    }

    fn minter2_addr() -> Addr {
        mock_addr("minter2")
    }

    fn user_addr() -> Addr {
        mock_addr("user")
    }

    fn minter_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_reply(crate::contract::reply);
        Box::new(contract)
    }

    fn collection_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            multi_creator_collection::contract::execute,
            multi_creator_collection::contract::instantiate,
            multi_creator_collection::contract::query,
        );
        Box::new(contract)
    }

    fn mock_app() -> App {
        AppBuilder::new().build(|_, _, _| {})
    }

    fn setup_contracts(app: &mut App) -> (u64, u64) {
        let minter_code_id = app.store_code(minter_contract());
        let collection_code_id = app.store_code(collection_contract());
        (minter_code_id, collection_code_id)
    }

    fn instantiate_minter(
        app: &mut App,
        minter_code_id: u64,
        collection_code_id: u64,
        initial_allowlist: Vec<String>,
        royalty_bps: u64,
    ) -> Addr {
        let msg = InstantiateMsg {
            collection_params: CollectionParams {
                code_id: collection_code_id,
                name: "Test Collection".to_string(),
                symbol: "TEST".to_string(),
                description: Some("A test collection".to_string()),
                image: Some("https://example.com/image.png".to_string()),
                external_link: None,
            },
            royalty_bps,
            initial_allowlist,
        };

        app.instantiate_contract(
            minter_code_id,
            admin_addr(),
            &msg,
            &[],
            "minter",
            Some(admin_addr().to_string()),
        )
        .unwrap()
    }

    // ==================== Instantiation Tests ====================

    #[test]
    fn test_instantiate() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string()],
            500,
        );

        // Query config
        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::Config {})
            .unwrap();

        assert_eq!(res.config.admin, admin_addr());
        assert_eq!(res.config.royalty_bps, 500);
        assert!(!res.config.is_paused);
        assert!(res.config.collection.is_some());

        // Check allowlist
        let res: IsAllowedResponse = app
            .wrap()
            .query_wasm_smart(
                &minter_addr,
                &QueryMsg::IsAllowed {
                    address: minter1_addr().to_string(),
                },
            )
            .unwrap();
        assert!(res.allowed);
    }

    #[test]
    fn test_instantiate_invalid_royalty() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let msg = InstantiateMsg {
            collection_params: CollectionParams {
                code_id: collection_code_id,
                name: "Test".to_string(),
                symbol: "TEST".to_string(),
                description: None,
                image: None,
                external_link: None,
            },
            royalty_bps: 1001, // Over 10%
            initial_allowlist: vec![],
        };

        let err = app
            .instantiate_contract(
                minter_code_id,
                admin_addr(),
                &msg,
                &[],
                "minter",
                None,
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Invalid royalty"));
    }

    // ==================== Minting Tests ====================

    #[test]
    fn test_mint_success() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string()],
            500,
        );

        // Mint
        let msg = ExecuteMsg::Mint {
            token_uri: "https://example.com/token/1.json".to_string(),
            use_per_token_royalty: None, // defaults to true
        };
        app.execute_contract(minter1_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        // Check token count
        let res: TokenCountResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::TokenCount {})
            .unwrap();
        assert_eq!(res.count, 1);
    }

    #[test]
    fn test_mint_ipfs_uri() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string()],
            500,
        );

        // Mint with IPFS URI
        let msg = ExecuteMsg::Mint {
            token_uri: "ipfs://QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG".to_string(),
            use_per_token_royalty: None,
        };
        app.execute_contract(minter1_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        let res: TokenCountResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::TokenCount {})
            .unwrap();
        assert_eq!(res.count, 1);
    }

    #[test]
    fn test_mint_not_on_allowlist() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string()],
            500,
        );

        let msg = ExecuteMsg::Mint {
            token_uri: "https://example.com/token/1.json".to_string(),
            use_per_token_royalty: None,
        };
        let err = app
            .execute_contract(user_addr(), minter_addr, &msg, &[])
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("not on allowlist"));
    }

    #[test]
    fn test_mint_when_paused() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string()],
            500,
        );

        // Pause minting
        let msg = ExecuteMsg::SetPaused { paused: true };
        app.execute_contract(admin_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        // Try to mint
        let msg = ExecuteMsg::Mint {
            token_uri: "https://example.com/token/1.json".to_string(),
            use_per_token_royalty: None,
        };
        let err = app
            .execute_contract(minter1_addr(), minter_addr, &msg, &[])
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("paused"));
    }

    #[test]
    fn test_mint_empty_uri() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string()],
            500,
        );

        let msg = ExecuteMsg::Mint {
            token_uri: "".to_string(),
            use_per_token_royalty: None,
        };
        let err = app
            .execute_contract(minter1_addr(), minter_addr, &msg, &[])
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Empty token URI"));
    }

    #[test]
    fn test_mint_invalid_uri_scheme() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string()],
            500,
        );

        let msg = ExecuteMsg::Mint {
            token_uri: "http://example.com/token/1.json".to_string(), // http not allowed
            use_per_token_royalty: None,
        };
        let err = app
            .execute_contract(minter1_addr(), minter_addr, &msg, &[])
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Invalid URI scheme"));
    }

    #[test]
    fn test_mint_invalid_uri() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string()],
            500,
        );

        let msg = ExecuteMsg::Mint {
            token_uri: "not a valid uri".to_string(),
            use_per_token_royalty: None,
        };
        let err = app
            .execute_contract(minter1_addr(), minter_addr, &msg, &[])
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Invalid token URI"));
    }

    #[test]
    fn test_mint_multiple_tokens() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string(), minter2_addr().to_string()],
            500,
        );

        // Minter1 mints
        for i in 1..=3 {
            let msg = ExecuteMsg::Mint {
                token_uri: format!("https://example.com/token/{}.json", i),
                use_per_token_royalty: None,
            };
            app.execute_contract(minter1_addr(), minter_addr.clone(), &msg, &[])
                .unwrap();
        }

        // Minter2 mints
        for i in 4..=5 {
            let msg = ExecuteMsg::Mint {
                token_uri: format!("https://example.com/token/{}.json", i),
                use_per_token_royalty: None,
            };
            app.execute_contract(minter2_addr(), minter_addr.clone(), &msg, &[])
                .unwrap();
        }

        let res: TokenCountResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::TokenCount {})
            .unwrap();
        assert_eq!(res.count, 5);
    }

    // ==================== Allowlist Tests ====================

    #[test]
    fn test_add_to_allowlist() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![],
            500,
        );

        // USER is not on allowlist initially
        let res: IsAllowedResponse = app
            .wrap()
            .query_wasm_smart(
                &minter_addr,
                &QueryMsg::IsAllowed {
                    address: user_addr().to_string(),
                },
            )
            .unwrap();
        assert!(!res.allowed);

        // Add USER to allowlist
        let msg = ExecuteMsg::AddToAllowlist {
            addresses: vec![user_addr().to_string()],
        };
        app.execute_contract(admin_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        // USER is now on allowlist
        let res: IsAllowedResponse = app
            .wrap()
            .query_wasm_smart(
                &minter_addr,
                &QueryMsg::IsAllowed {
                    address: user_addr().to_string(),
                },
            )
            .unwrap();
        assert!(res.allowed);
    }

    #[test]
    fn test_add_to_allowlist_unauthorized() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![],
            500,
        );

        let msg = ExecuteMsg::AddToAllowlist {
            addresses: vec![user_addr().to_string()],
        };
        let err = app
            .execute_contract(user_addr(), minter_addr, &msg, &[])
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn test_remove_from_allowlist() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string()],
            500,
        );

        // Remove MINTER1 from allowlist
        let msg = ExecuteMsg::RemoveFromAllowlist {
            addresses: vec![minter1_addr().to_string()],
        };
        app.execute_contract(admin_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        // MINTER1 is no longer on allowlist
        let res: IsAllowedResponse = app
            .wrap()
            .query_wasm_smart(
                &minter_addr,
                &QueryMsg::IsAllowed {
                    address: minter1_addr().to_string(),
                },
            )
            .unwrap();
        assert!(!res.allowed);

        // MINTER1 can no longer mint
        let msg = ExecuteMsg::Mint {
            token_uri: "https://example.com/token/1.json".to_string(),
            use_per_token_royalty: None,
        };
        let err = app
            .execute_contract(minter1_addr(), minter_addr, &msg, &[])
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("not on allowlist"));
    }

    #[test]
    fn test_remove_from_allowlist_unauthorized() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string()],
            500,
        );

        let msg = ExecuteMsg::RemoveFromAllowlist {
            addresses: vec![minter1_addr().to_string()],
        };
        let err = app
            .execute_contract(minter1_addr(), minter_addr, &msg, &[])
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    // ==================== Pause Tests ====================

    #[test]
    fn test_set_paused() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string()],
            500,
        );

        // Pause
        let msg = ExecuteMsg::SetPaused { paused: true };
        app.execute_contract(admin_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::Config {})
            .unwrap();
        assert!(res.config.is_paused);

        // Unpause
        let msg = ExecuteMsg::SetPaused { paused: false };
        app.execute_contract(admin_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::Config {})
            .unwrap();
        assert!(!res.config.is_paused);
    }

    #[test]
    fn test_set_paused_unauthorized() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![],
            500,
        );

        let msg = ExecuteMsg::SetPaused { paused: true };
        let err = app
            .execute_contract(user_addr(), minter_addr, &msg, &[])
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    // ==================== Royalty Tests ====================

    #[test]
    fn test_update_royalty() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![],
            500,
        );

        // Update royalty to 7.5%
        let msg = ExecuteMsg::UpdateRoyalty { royalty_bps: 750 };
        app.execute_contract(admin_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::Config {})
            .unwrap();
        assert_eq!(res.config.royalty_bps, 750);
    }

    #[test]
    fn test_update_royalty_invalid() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![],
            500,
        );

        let msg = ExecuteMsg::UpdateRoyalty { royalty_bps: 1001 };
        let err = app
            .execute_contract(admin_addr(), minter_addr, &msg, &[])
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Invalid royalty"));
    }

    #[test]
    fn test_update_royalty_unauthorized() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![],
            500,
        );

        let msg = ExecuteMsg::UpdateRoyalty { royalty_bps: 750 };
        let err = app
            .execute_contract(user_addr(), minter_addr, &msg, &[])
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    // ==================== Admin Tests ====================

    #[test]
    fn test_update_admin() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![],
            500,
        );

        // Transfer admin
        let msg = ExecuteMsg::UpdateAdmin {
            new_admin: user_addr().to_string(),
        };
        app.execute_contract(admin_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::Config {})
            .unwrap();
        assert_eq!(res.config.admin, user_addr());

        // Old admin can no longer update settings
        let msg = ExecuteMsg::SetPaused { paused: true };
        let err = app
            .execute_contract(admin_addr(), minter_addr.clone(), &msg, &[])
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Unauthorized"));

        // New admin can update settings
        let msg = ExecuteMsg::SetPaused { paused: true };
        app.execute_contract(user_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::Config {})
            .unwrap();
        assert!(res.config.is_paused);
    }

    #[test]
    fn test_update_admin_unauthorized() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![],
            500,
        );

        let msg = ExecuteMsg::UpdateAdmin {
            new_admin: user_addr().to_string(),
        };
        let err = app
            .execute_contract(user_addr(), minter_addr, &msg, &[])
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    // ==================== Integration Tests ====================

    #[test]
    fn test_minted_token_has_correct_royalty_info() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string()],
            500, // 5%
        );

        // Get collection address
        let config: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::Config {})
            .unwrap();
        let collection_addr = config.config.collection.unwrap();

        // Mint (default: per-token royalty enabled)
        let msg = ExecuteMsg::Mint {
            token_uri: "https://example.com/token/1.json".to_string(),
            use_per_token_royalty: None, // defaults to true
        };
        app.execute_contract(minter1_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        // Query royalty info from collection
        let royalty_res: multi_creator_collection::msg::RoyaltyInfoResponse = app
            .wrap()
            .query_wasm_smart(
                &collection_addr,
                &multi_creator_collection::msg::QueryMsg::RoyaltyInfo {
                    token_id: Some("1".to_string()),
                    sale_price: cosmwasm_std::Uint128::new(10000),
                },
            )
            .unwrap();

        assert_eq!(royalty_res.address, minter1_addr().to_string()); // Creator is the minter
        assert_eq!(royalty_res.royalty_amount, cosmwasm_std::Uint128::new(500)); // 5% of 10000
    }

    #[test]
    fn test_different_minters_get_different_royalty_addresses() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string(), minter2_addr().to_string()],
            500,
        );

        let config: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::Config {})
            .unwrap();
        let collection_addr = config.config.collection.unwrap();

        // MINTER1 mints token 1
        let msg = ExecuteMsg::Mint {
            token_uri: "https://example.com/token/1.json".to_string(),
            use_per_token_royalty: None,
        };
        app.execute_contract(minter1_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        // MINTER2 mints token 2
        let msg = ExecuteMsg::Mint {
            token_uri: "https://example.com/token/2.json".to_string(),
            use_per_token_royalty: None,
        };
        app.execute_contract(minter2_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        // Token 1 royalty goes to MINTER1
        let royalty1: multi_creator_collection::msg::RoyaltyInfoResponse = app
            .wrap()
            .query_wasm_smart(
                &collection_addr,
                &multi_creator_collection::msg::QueryMsg::RoyaltyInfo {
                    token_id: Some("1".to_string()),
                    sale_price: cosmwasm_std::Uint128::new(1000),
                },
            )
            .unwrap();
        assert_eq!(royalty1.address, minter1_addr().to_string());

        // Token 2 royalty goes to MINTER2
        let royalty2: multi_creator_collection::msg::RoyaltyInfoResponse = app
            .wrap()
            .query_wasm_smart(
                &collection_addr,
                &multi_creator_collection::msg::QueryMsg::RoyaltyInfo {
                    token_id: Some("2".to_string()),
                    sale_price: cosmwasm_std::Uint128::new(1000),
                },
            )
            .unwrap();
        assert_eq!(royalty2.address, minter2_addr().to_string());
    }

    #[test]
    fn test_royalty_rate_change_affects_new_tokens_only() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string()],
            500, // 5%
        );

        let config: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::Config {})
            .unwrap();
        let collection_addr = config.config.collection.unwrap();

        // Mint token 1 with 5% royalty
        let msg = ExecuteMsg::Mint {
            token_uri: "https://example.com/token/1.json".to_string(),
            use_per_token_royalty: None,
        };
        app.execute_contract(minter1_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        // Update royalty to 10%
        let msg = ExecuteMsg::UpdateRoyalty { royalty_bps: 1000 };
        app.execute_contract(admin_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        // Mint token 2 with 10% royalty
        let msg = ExecuteMsg::Mint {
            token_uri: "https://example.com/token/2.json".to_string(),
            use_per_token_royalty: None,
        };
        app.execute_contract(minter1_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        // Token 1 should still have 5% royalty
        let royalty1: multi_creator_collection::msg::RoyaltyInfoResponse = app
            .wrap()
            .query_wasm_smart(
                &collection_addr,
                &multi_creator_collection::msg::QueryMsg::RoyaltyInfo {
                    token_id: Some("1".to_string()),
                    sale_price: cosmwasm_std::Uint128::new(10000),
                },
            )
            .unwrap();
        assert_eq!(royalty1.royalty_amount, cosmwasm_std::Uint128::new(500)); // 5%

        // Token 2 should have 10% royalty
        let royalty2: multi_creator_collection::msg::RoyaltyInfoResponse = app
            .wrap()
            .query_wasm_smart(
                &collection_addr,
                &multi_creator_collection::msg::QueryMsg::RoyaltyInfo {
                    token_id: Some("2".to_string()),
                    sale_price: cosmwasm_std::Uint128::new(10000),
                },
            )
            .unwrap();
        assert_eq!(royalty2.royalty_amount, cosmwasm_std::Uint128::new(1000)); // 10%
    }

    // ==================== Two-Tier Royalty Tests ====================

    #[test]
    fn test_mint_with_general_royalty_fallback() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string()],
            500, // 5%
        );

        let config: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::Config {})
            .unwrap();
        let collection_addr = config.config.collection.unwrap();

        // Mint token WITHOUT per-token royalty (use general)
        let msg = ExecuteMsg::Mint {
            token_uri: "https://example.com/token/1.json".to_string(),
            use_per_token_royalty: Some(false),
        };
        app.execute_contract(minter1_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        // Query royalty - should return general royalty (admin is the recipient)
        let royalty_res: multi_creator_collection::msg::RoyaltyInfoResponse = app
            .wrap()
            .query_wasm_smart(
                &collection_addr,
                &multi_creator_collection::msg::QueryMsg::RoyaltyInfo {
                    token_id: Some("1".to_string()),
                    sale_price: cosmwasm_std::Uint128::new(10000),
                },
            )
            .unwrap();

        // General royalty recipient is the admin
        assert_eq!(royalty_res.address, admin_addr().to_string());
        assert_eq!(royalty_res.royalty_amount, cosmwasm_std::Uint128::new(500)); // 5% of 10000
    }

    #[test]
    fn test_general_royalty_query_from_minter() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string()],
            500, // 5%
        );

        let config: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::Config {})
            .unwrap();
        let collection_addr = config.config.collection.unwrap();

        // Query general royalty
        let general_royalty: multi_creator_collection::msg::GeneralRoyaltyInfoResponse = app
            .wrap()
            .query_wasm_smart(
                &collection_addr,
                &multi_creator_collection::msg::QueryMsg::GeneralRoyalty {},
            )
            .unwrap();

        assert!(general_royalty.general_royalty.is_some());
        let gr = general_royalty.general_royalty.unwrap();
        assert_eq!(gr.address, admin_addr().to_string());
        assert_eq!(gr.royalty_bps, 500);
    }

    #[test]
    fn test_royalty_query_without_token_id_returns_general() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string()],
            700, // 7%
        );

        let config: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::Config {})
            .unwrap();
        let collection_addr = config.config.collection.unwrap();

        // Query royalty without token_id
        let royalty_res: multi_creator_collection::msg::RoyaltyInfoResponse = app
            .wrap()
            .query_wasm_smart(
                &collection_addr,
                &multi_creator_collection::msg::QueryMsg::RoyaltyInfo {
                    token_id: None,
                    sale_price: cosmwasm_std::Uint128::new(10000),
                },
            )
            .unwrap();

        assert_eq!(royalty_res.address, admin_addr().to_string());
        assert_eq!(royalty_res.royalty_amount, cosmwasm_std::Uint128::new(700)); // 7% of 10000
    }

    #[test]
    fn test_mixed_per_token_and_general_royalties() {
        let mut app = mock_app();
        let (minter_code_id, collection_code_id) = setup_contracts(&mut app);

        let minter_addr = instantiate_minter(
            &mut app,
            minter_code_id,
            collection_code_id,
            vec![minter1_addr().to_string(), minter2_addr().to_string()],
            500, // 5% general royalty
        );

        let config: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&minter_addr, &QueryMsg::Config {})
            .unwrap();
        let collection_addr = config.config.collection.unwrap();

        // MINTER1 mints with per-token royalty
        let msg = ExecuteMsg::Mint {
            token_uri: "https://example.com/token/1.json".to_string(),
            use_per_token_royalty: Some(true),
        };
        app.execute_contract(minter1_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        // MINTER2 mints without per-token royalty (uses general)
        let msg = ExecuteMsg::Mint {
            token_uri: "https://example.com/token/2.json".to_string(),
            use_per_token_royalty: Some(false),
        };
        app.execute_contract(minter2_addr(), minter_addr.clone(), &msg, &[])
            .unwrap();

        // Token 1: per-token royalty goes to MINTER1
        let royalty1: multi_creator_collection::msg::RoyaltyInfoResponse = app
            .wrap()
            .query_wasm_smart(
                &collection_addr,
                &multi_creator_collection::msg::QueryMsg::RoyaltyInfo {
                    token_id: Some("1".to_string()),
                    sale_price: cosmwasm_std::Uint128::new(10000),
                },
            )
            .unwrap();
        assert_eq!(royalty1.address, minter1_addr().to_string());
        assert_eq!(royalty1.royalty_amount, cosmwasm_std::Uint128::new(500)); // 5%

        // Token 2: general royalty goes to ADMIN
        let royalty2: multi_creator_collection::msg::RoyaltyInfoResponse = app
            .wrap()
            .query_wasm_smart(
                &collection_addr,
                &multi_creator_collection::msg::QueryMsg::RoyaltyInfo {
                    token_id: Some("2".to_string()),
                    sale_price: cosmwasm_std::Uint128::new(10000),
                },
            )
            .unwrap();
        assert_eq!(royalty2.address, admin_addr().to_string());
        assert_eq!(royalty2.royalty_amount, cosmwasm_std::Uint128::new(500)); // 5%
    }
}
