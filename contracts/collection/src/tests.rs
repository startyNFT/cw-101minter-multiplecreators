#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi};
    use cosmwasm_std::{from_json, Addr, Timestamp, Uint128};

    use crate::contract::{execute, instantiate, query};
    use crate::error::ContractError;
    use crate::msg::{
        AllNftInfoResponse, CheckRoyaltiesResponse, CollectionExtension, CollectionInfoResponse,
        ContractInfoResponse, ExecuteMsg, InstantiateMsg, MinterResponse, NftInfoResponse,
        QueryMsg, RoyaltyInfoResponse, TokenExtension,
    };
    use cw721::msg::{NumTokensResponse, OwnerOfResponse, TokensResponse};
    use cw721::Expiration;

    fn mock_addr(name: &str) -> Addr {
        MockApi::default().addr_make(name)
    }

    fn minter_addr() -> Addr {
        mock_addr("minter")
    }

    fn creator_addr() -> Addr {
        mock_addr("creator")
    }

    fn owner_addr() -> Addr {
        mock_addr("owner")
    }

    fn spender_addr() -> Addr {
        mock_addr("spender")
    }

    fn operator_addr() -> Addr {
        mock_addr("operator")
    }

    fn recipient_addr() -> Addr {
        mock_addr("recipient")
    }

    fn other_owner_addr() -> Addr {
        mock_addr("other_owner")
    }

    fn other_creator_addr() -> Addr {
        mock_addr("other_creator")
    }

    fn random_user_addr() -> Addr {
        mock_addr("random_user")
    }

    fn setup_contract(deps: cosmwasm_std::DepsMut) {
        let msg = InstantiateMsg {
            name: "Test Collection".to_string(),
            symbol: "TEST".to_string(),
            minter: minter_addr().to_string(),
            creator: Some(creator_addr().to_string()),
            collection_info: Some(CollectionExtension {
                description: Some("A test collection".to_string()),
                image: Some("https://example.com/image.png".to_string()),
                external_link: Some("https://example.com".to_string()),
                explicit_content: Some(false),
                start_trading_time: None,
            }),
        };
        let info = message_info(&creator_addr(), &[]);
        instantiate(deps, mock_env(), info, msg).unwrap();
    }

    fn mint_token(
        deps: cosmwasm_std::DepsMut,
        token_id: &str,
        owner: &Addr,
        creator: &Addr,
        royalty_bps: u64,
    ) {
        let msg = ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: owner.to_string(),
            token_uri: Some("https://example.com/token/1".to_string()),
            extension: TokenExtension {
                creator: Some(creator.to_string()),
                royalty_bps: Some(royalty_bps),
                minted_at: Some(Timestamp::from_seconds(1000000)),
            },
        };
        let info = message_info(&minter_addr(), &[]);
        execute(deps, mock_env(), info, msg).unwrap();
    }

    // ==================== Instantiation Tests ====================

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Query contract info
        let res: ContractInfoResponse =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::ContractInfo {}).unwrap())
                .unwrap();
        assert_eq!(res.name, "Test Collection");
        assert_eq!(res.symbol, "TEST");

        // Query minter
        let res: MinterResponse =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::Minter {}).unwrap()).unwrap();
        assert_eq!(res.minter, Some(minter_addr().to_string()));

        // Query collection info
        let res: CollectionInfoResponse =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::CollectionInfo {}).unwrap())
                .unwrap();
        assert_eq!(res.name, "Test Collection");
        assert_eq!(res.symbol, "TEST");
        assert!(res.extension.is_some());
        let ext = res.extension.unwrap();
        assert_eq!(ext.description, Some("A test collection".to_string()));
    }

    #[test]
    fn test_instantiate_without_collection_info() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            name: "Minimal Collection".to_string(),
            symbol: "MIN".to_string(),
            minter: minter_addr().to_string(),
            creator: None,
            collection_info: None,
        };
        let info = message_info(&creator_addr(), &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let res: CollectionInfoResponse =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::CollectionInfo {}).unwrap())
                .unwrap();
        assert_eq!(res.name, "Minimal Collection");
        assert!(res.extension.is_none());
    }

    // ==================== Minting Tests ====================

    #[test]
    fn test_mint() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 500);

        // Check token count
        let res: NumTokensResponse =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::NumTokens {}).unwrap()).unwrap();
        assert_eq!(res.count, 1);

        // Check owner
        let res: OwnerOfResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::OwnerOf {
                    token_id: "1".to_string(),
                    include_expired: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(res.owner, owner_addr().to_string());

        // Check NFT info with extension
        let res: NftInfoResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::NftInfo {
                    token_id: "1".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            res.token_uri,
            Some("https://example.com/token/1".to_string())
        );
        assert_eq!(res.extension.creator, Some(creator_addr().to_string()));
        assert_eq!(res.extension.royalty_bps, Some(500));
    }

    #[test]
    fn test_mint_unauthorized() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let msg = ExecuteMsg::Mint {
            token_id: "1".to_string(),
            owner: owner_addr().to_string(),
            token_uri: None,
            extension: TokenExtension::default(),
        };
        let info = message_info(&random_user_addr(), &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::Unauthorized("Only minter can mint".to_string())
        );
    }

    #[test]
    fn test_mint_duplicate_token() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 500);

        let msg = ExecuteMsg::Mint {
            token_id: "1".to_string(),
            owner: owner_addr().to_string(),
            token_uri: None,
            extension: TokenExtension::default(),
        };
        let info = message_info(&minter_addr(), &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::TokenAlreadyExists {
                token_id: "1".to_string()
            }
        );
    }

    // ==================== Transfer Tests ====================

    #[test]
    fn test_transfer() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 500);

        let msg = ExecuteMsg::TransferNft {
            recipient: recipient_addr().to_string(),
            token_id: "1".to_string(),
        };
        let info = message_info(&owner_addr(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Verify new owner
        let res: OwnerOfResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::OwnerOf {
                    token_id: "1".to_string(),
                    include_expired: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(res.owner, recipient_addr().to_string());
    }

    #[test]
    fn test_transfer_not_owner() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 500);

        let msg = ExecuteMsg::TransferNft {
            recipient: recipient_addr().to_string(),
            token_id: "1".to_string(),
        };
        let info = message_info(&random_user_addr(), &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::NotOwnerOrApproved {});
    }

    #[test]
    fn test_transfer_nonexistent_token() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let msg = ExecuteMsg::TransferNft {
            recipient: recipient_addr().to_string(),
            token_id: "999".to_string(),
        };
        let info = message_info(&owner_addr(), &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::TokenNotFound {
                token_id: "999".to_string()
            }
        );
    }

    // ==================== Approval Tests ====================

    #[test]
    fn test_approve_and_transfer() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 500);

        // Approve spender
        let msg = ExecuteMsg::Approve {
            spender: spender_addr().to_string(),
            token_id: "1".to_string(),
            expires: None,
        };
        let info = message_info(&owner_addr(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Spender can transfer
        let msg = ExecuteMsg::TransferNft {
            recipient: recipient_addr().to_string(),
            token_id: "1".to_string(),
        };
        let info = message_info(&spender_addr(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Verify new owner
        let res: OwnerOfResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::OwnerOf {
                    token_id: "1".to_string(),
                    include_expired: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(res.owner, recipient_addr().to_string());
    }

    #[test]
    fn test_approve_not_owner() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 500);

        let msg = ExecuteMsg::Approve {
            spender: spender_addr().to_string(),
            token_id: "1".to_string(),
            expires: None,
        };
        let info = message_info(&random_user_addr(), &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::Unauthorized("Only owner can approve".to_string())
        );
    }

    #[test]
    fn test_revoke() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 500);

        // Approve
        let msg = ExecuteMsg::Approve {
            spender: spender_addr().to_string(),
            token_id: "1".to_string(),
            expires: None,
        };
        let info = message_info(&owner_addr(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Revoke
        let msg = ExecuteMsg::Revoke {
            spender: spender_addr().to_string(),
            token_id: "1".to_string(),
        };
        let info = message_info(&owner_addr(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Spender can no longer transfer
        let msg = ExecuteMsg::TransferNft {
            recipient: recipient_addr().to_string(),
            token_id: "1".to_string(),
        };
        let info = message_info(&spender_addr(), &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::NotOwnerOrApproved {});
    }

    #[test]
    fn test_approval_expires() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 500);

        // Approve with expiration at height 100
        let msg = ExecuteMsg::Approve {
            spender: spender_addr().to_string(),
            token_id: "1".to_string(),
            expires: Some(Expiration::AtHeight(100)),
        };
        let info = message_info(&owner_addr(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Transfer fails after expiration
        let mut env = mock_env();
        env.block.height = 101;
        let msg = ExecuteMsg::TransferNft {
            recipient: recipient_addr().to_string(),
            token_id: "1".to_string(),
        };
        let info = message_info(&spender_addr(), &[]);
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::NotOwnerOrApproved {});
    }

    // ==================== Operator Tests ====================

    #[test]
    fn test_approve_all() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 500);
        mint_token(deps.as_mut(), "2", &owner_addr(), &creator_addr(), 500);

        // Approve all for operator
        let msg = ExecuteMsg::ApproveAll {
            operator: operator_addr().to_string(),
            expires: None,
        };
        let info = message_info(&owner_addr(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Operator can transfer any token
        let msg = ExecuteMsg::TransferNft {
            recipient: recipient_addr().to_string(),
            token_id: "1".to_string(),
        };
        let info = message_info(&operator_addr(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::TransferNft {
            recipient: recipient_addr().to_string(),
            token_id: "2".to_string(),
        };
        let info = message_info(&operator_addr(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Verify both transferred
        let res: TokensResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Tokens {
                    owner: recipient_addr().to_string(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(res.tokens.len(), 2);
    }

    #[test]
    fn test_revoke_all() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 500);

        // Approve all
        let msg = ExecuteMsg::ApproveAll {
            operator: operator_addr().to_string(),
            expires: None,
        };
        let info = message_info(&owner_addr(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Revoke all
        let msg = ExecuteMsg::RevokeAll {
            operator: operator_addr().to_string(),
        };
        let info = message_info(&owner_addr(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Operator can no longer transfer
        let msg = ExecuteMsg::TransferNft {
            recipient: recipient_addr().to_string(),
            token_id: "1".to_string(),
        };
        let info = message_info(&operator_addr(), &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::NotOwnerOrApproved {});
    }

    // ==================== Burn Tests ====================

    #[test]
    fn test_burn() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 500);

        let msg = ExecuteMsg::Burn {
            token_id: "1".to_string(),
        };
        let info = message_info(&owner_addr(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Token count should be 0
        let res: NumTokensResponse =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::NumTokens {}).unwrap()).unwrap();
        assert_eq!(res.count, 0);

        // Token should not exist
        let err = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::NftInfo {
                token_id: "1".to_string(),
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_burn_not_owner() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 500);

        let msg = ExecuteMsg::Burn {
            token_id: "1".to_string(),
        };
        let info = message_info(&random_user_addr(), &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::Unauthorized("Only owner can burn".to_string())
        );
    }

    // ==================== Royalty Tests (CW2981) ====================

    #[test]
    fn test_royalty_info() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 500); // 5%

        // Query royalty for sale of 1000
        let res: RoyaltyInfoResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::RoyaltyInfo {
                    token_id: "1".to_string(),
                    sale_price: Uint128::new(1000),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(res.address, creator_addr().to_string());
        assert_eq!(res.royalty_amount, Uint128::new(50)); // 5% of 1000
    }

    #[test]
    fn test_royalty_info_different_rates() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Mint with 10% royalty
        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 1000);
        // Mint with 2.5% royalty
        mint_token(deps.as_mut(), "2", &owner_addr(), &other_creator_addr(), 250);

        let res1: RoyaltyInfoResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::RoyaltyInfo {
                    token_id: "1".to_string(),
                    sale_price: Uint128::new(10000),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(res1.royalty_amount, Uint128::new(1000)); // 10%

        let res2: RoyaltyInfoResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::RoyaltyInfo {
                    token_id: "2".to_string(),
                    sale_price: Uint128::new(10000),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(res2.address, other_creator_addr().to_string());
        assert_eq!(res2.royalty_amount, Uint128::new(250)); // 2.5%
    }

    #[test]
    fn test_royalty_info_zero_royalty() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 0); // 0%

        let res: RoyaltyInfoResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::RoyaltyInfo {
                    token_id: "1".to_string(),
                    sale_price: Uint128::new(1000),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(res.royalty_amount, Uint128::zero());
    }

    #[test]
    fn test_check_royalties() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let res: CheckRoyaltiesResponse =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::CheckRoyalties {}).unwrap())
                .unwrap();

        assert!(res.royalty_payments);
    }

    // ==================== Query Tests ====================

    #[test]
    fn test_all_nft_info() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 500);

        let res: AllNftInfoResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AllNftInfo {
                    token_id: "1".to_string(),
                    include_expired: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(res.access.owner, owner_addr().to_string());
        assert_eq!(
            res.info.token_uri,
            Some("https://example.com/token/1".to_string())
        );
        assert_eq!(res.info.extension.royalty_bps, Some(500));
    }

    #[test]
    fn test_all_tokens() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 500);
        mint_token(deps.as_mut(), "2", &owner_addr(), &creator_addr(), 500);
        mint_token(deps.as_mut(), "3", &other_owner_addr(), &creator_addr(), 500);

        let res: TokensResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AllTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(res.tokens.len(), 3);
    }

    #[test]
    fn test_tokens_by_owner() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 500);
        mint_token(deps.as_mut(), "2", &owner_addr(), &creator_addr(), 500);
        mint_token(deps.as_mut(), "3", &other_owner_addr(), &creator_addr(), 500);

        let res: TokensResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Tokens {
                    owner: owner_addr().to_string(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(res.tokens.len(), 2);
        assert!(res.tokens.contains(&"1".to_string()));
        assert!(res.tokens.contains(&"2".to_string()));
    }

    #[test]
    fn test_all_tokens_pagination() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        for i in 1..=5 {
            mint_token(deps.as_mut(), &i.to_string(), &owner_addr(), &creator_addr(), 500);
        }

        // First page
        let res: TokensResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AllTokens {
                    start_after: None,
                    limit: Some(2),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(res.tokens.len(), 2);

        // Second page
        let res2: TokensResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AllTokens {
                    start_after: Some(res.tokens.last().unwrap().clone()),
                    limit: Some(2),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(res2.tokens.len(), 2);
    }

    // ==================== Update Minter Tests ====================

    #[test]
    fn test_update_minter() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let new_minter = mock_addr("new_minter");
        let msg = ExecuteMsg::UpdateMinter {
            new_minter: Some(new_minter.to_string()),
        };
        let info = message_info(&minter_addr(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let res: MinterResponse =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::Minter {}).unwrap()).unwrap();
        assert_eq!(res.minter, Some(new_minter.to_string()));
    }

    #[test]
    fn test_update_minter_to_none() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let msg = ExecuteMsg::UpdateMinter { new_minter: None };
        let info = message_info(&minter_addr(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let res: MinterResponse =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::Minter {}).unwrap()).unwrap();
        assert_eq!(res.minter, None);
    }

    #[test]
    fn test_update_minter_unauthorized() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let new_minter = mock_addr("new_minter");
        let msg = ExecuteMsg::UpdateMinter {
            new_minter: Some(new_minter.to_string()),
        };
        let info = message_info(&random_user_addr(), &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::Unauthorized("Only minter can update minter".to_string())
        );
    }

    // ==================== Transfer Clears Approvals ====================

    #[test]
    fn test_transfer_clears_approvals() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        mint_token(deps.as_mut(), "1", &owner_addr(), &creator_addr(), 500);

        // Approve spender
        let msg = ExecuteMsg::Approve {
            spender: spender_addr().to_string(),
            token_id: "1".to_string(),
            expires: None,
        };
        let info = message_info(&owner_addr(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Transfer token
        let msg = ExecuteMsg::TransferNft {
            recipient: recipient_addr().to_string(),
            token_id: "1".to_string(),
        };
        let info = message_info(&owner_addr(), &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Old spender cannot transfer anymore
        let msg = ExecuteMsg::TransferNft {
            recipient: owner_addr().to_string(),
            token_id: "1".to_string(),
        };
        let info = message_info(&spender_addr(), &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::NotOwnerOrApproved {});
    }
}
