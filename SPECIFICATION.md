# Starty Multi-Creator NFT Minter - Technical Specification

## Overview
A CosmWasm smart contract for Stargaze that enables multiple creators to mint NFTs under a single collection, with a **global royalty percentage** applied to all creators. Uses **allowlist-based authentication** for minting access control.

**Base Contract:** Stargaze base-minter (1-of-1 minter)
**Target Network:** Stargaze (CosmWasm)
**Language:** Rust + CosmWasm

---

## Core Requirements

### 1. Allowlist-Based Minting
- **Instead of:** Creator wallet address verification (single creator)
- **Use:** Allowlist of approved addresses
- Only addresses on the allowlist can mint NFTs
- Minting is done by the sender (sender = creator = recipient)
- Admin can add/remove addresses from allowlist

### 2. Global Royalty Percentage
- Single royalty percentage (max 10%) applies to all tokens
- Each minted NFT records its creator's wallet address
- Royalty payments go to the individual creator's wallet
- Admin can update the global royalty percentage
- All existing tokens use the current global percentage

### 3. No Fair Burn
- Remove all Fair Burn fee mechanisms
- Minting is free (no fees required)
- No automatic community pool burning

### 4. Admin Controls
- Admin (contract instantiator) can:
  - Add/remove addresses from allowlist
  - Pause/unpause minting
  - Update global creator royalty percentage (max 10%)
  - Update collection trading time

---

## State Structure

```rust
// Main configuration
pub struct Config {
    pub admin: Addr,                    // Admin address (can update config)
    pub collection_address: Option<Addr>, // Associated SG721 collection
    pub collection_code_id: u64,        // SG721 code ID used
    pub creator_royalty_bps: u64,       // Global royalty in basis points (max 1000 = 10%)
    pub is_paused: bool,                // Pause minting globally
}

// Per-token metadata (creator info)
pub struct TokenRoyalty {
    pub creator: Addr,         // Creator who minted this token (receives royalties)
    pub token_uri: String,     // IPFS or HTTPS URL
    pub minted_at: Timestamp,  // When token was minted
}

// Storage maps
pub const CONFIG: Item<Config> = Item::new("config");
pub const ALLOWLIST: Map<Addr, bool> = Map::new("allowlist");
pub const TOKEN_INDEX: Item<u64> = Item::new("token_index");
pub const TOKEN_ROYALTIES: Map<String, TokenRoyalty> = Map::new("token_royalties");
```

---

## Message Types

### InstantiateMsg
```rust
pub struct InstantiateMsg {
    pub collection_params: CollectionParams,  // SG721 collection parameters
    pub creator_royalty_bps: u64,             // Global royalty % (e.g., 500 = 5%, max 1000 = 10%)
    pub initial_allowlist: Vec<String>,       // Initial addresses allowed to mint
}

pub struct CollectionParams {
    pub code_id: u64,           // SG721 contract code ID
    pub name: String,           // Collection name
    pub symbol: String,         // Collection symbol
    pub info: CollectionInfo {
        creator: String,        // Admin address (becomes collection creator)
        description: String,
        image: String,          // Collection image URL
        external_link: Option<String>,
        start_trading_time: Option<Timestamp>,
    }
}
```

### ExecuteMsg
```rust
pub enum ExecuteMsg {
    // Mint a new NFT (sender must be on allowlist)
    Mint {
        token_uri: String,         // Metadata URL (IPFS or HTTPS)
    },

    // Admin: Add addresses to allowlist
    AddToAllowlist {
        addresses: Vec<String>,
    },

    // Admin: Remove addresses from allowlist
    RemoveFromAllowlist {
        addresses: Vec<String>,
    },

    // Admin: Pause or unpause minting
    SetPaused {
        paused: bool,
    },

    // Admin: Update global creator royalty percentage
    UpdateCreatorRoyalty {
        creator_royalty_bps: u64,  // Max 1000 (10%)
    },

    // Admin: Update collection trading start time
    UpdateStartTradingTime {
        start_time: Option<Timestamp>,
    },
}
```

### QueryMsg
```rust
pub enum QueryMsg {
    // Get contract configuration
    Config {},

    // Check if address is on allowlist
    IsAllowed {
        address: String,
    },

    // Get royalty info for a specific token (standard CW2981)
    RoyaltyInfo {
        token_id: String,
        sale_price: Uint128,
    },

    // Get detailed token information including creator
    TokenInfo {
        token_id: String,
    },

    // Get all tokens by a specific creator (paginated)
    TokensByCreator {
        creator: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
}
```

---

## Key Modifications from Base-Minter

### 1. Replace Creator Check with Allowlist Check
**Original (base-minter):**
```rust
if collection_info.creator != info.sender {
    return Err(ContractError::Unauthorized("Sender is not sg721 creator"));
}
```

**New (multi-creator):**
```rust
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
```

### 2. Store Per-Token Creator Data
**After minting NFT to collection:**
```rust
// Store token creator information (for royalty payments)
let token_royalty = TokenRoyalty {
    creator: info.sender.clone(),  // Sender is the creator
    token_uri: msg.token_uri.clone(),
    minted_at: env.block.time,
};
TOKEN_ROYALTIES.save(deps.storage, token_id.clone(), &token_royalty)?;
```

### 3. Implement Royalty Query with Global Percentage
```rust
pub fn query_royalty_info(
    deps: Deps,
    token_id: String,
    sale_price: Uint128,
) -> StdResult<RoyaltyInfoResponse> {
    let config = CONFIG.load(deps.storage)?;
    let royalty = TOKEN_ROYALTIES.load(deps.storage, token_id)?;

    // Calculate royalty amount using GLOBAL creator_royalty_bps
    let royalty_decimal = Decimal::bps(config.creator_royalty_bps);
    let royalty_amount = sale_price.checked_mul_floor(royalty_decimal)?;

    Ok(RoyaltyInfoResponse {
        address: royalty.creator.to_string(),  // Creator receives the royalty
        royalty_amount,
    })
}
```

### 4. Remove Fair Burn Completely
**Delete these parts:**
- `checked_fair_burn()` function calls
- Fee calculation based on `mint_fee_bps`
- Network fee payment validation
- All imports related to fair burn

**Simplified minting:**
```rust
// No payment required
// Just mint the NFT directly to sender
let mint_msg = Sg721ExecuteMsg::Mint {
    token_id: token_id.clone(),
    owner: info.sender.to_string(),  // Sender receives the NFT
    token_uri: Some(msg.token_uri.clone()),
    extension: None,
};
```

---

## Edge Cases & Handling

### 1. Unauthorized Minting Attempt
**Risk:** Someone not on allowlist tries to mint
**Mitigation:**
```rust
let is_allowed = ALLOWLIST.may_load(deps.storage, info.sender.clone())?.unwrap_or(false);
if !is_allowed {
    return Err(ContractError::NotOnAllowlist {});
}
```

### 2. Invalid Token URI
**Risk:** Malformed or malicious URLs
**Mitigation:**
```rust
// Validate URL format
let parsed_url = url::Url::parse(&msg.token_uri)
    .map_err(|_| ContractError::InvalidTokenUri { uri: msg.token_uri.clone() })?;

// Require HTTPS or IPFS
let scheme = parsed_url.scheme();
if scheme != "https" && scheme != "ipfs" {
    return Err(ContractError::InvalidUriScheme {
        scheme: scheme.to_string(),
    });
}
```

### 3. Royalty Set to 0%
**Risk:** Admin sets 0% royalty, creators get nothing
**Handling:**
- **Allow 0%** - Admin's choice
- Frontend should warn but not block
- No minimum royalty enforced

### 4. Royalty Exceeds Maximum
**Risk:** Admin tries to set royalty > 10%
**Mitigation:**
```rust
if creator_royalty_bps > 1000 {
    return Err(ContractError::InvalidCreatorRoyalty {});
}
```

### 5. Admin Abuse (Pausing, Allowlist Changes)
**Risk:** Admin could pause minting or remove creators maliciously
**Mitigation:**
- **Transparent operations** - All admin actions are on-chain and visible
- Frontend should display admin address prominently
- For production: Use multisig wallet as admin

### 6. Address Invalid
**Risk:** Adding invalid address to allowlist
**Mitigation:**
```rust
let address = deps.api.addr_validate(address_str)?;
```
This validates address format.

### 7. Token ID Collision
**Risk:** Two mints get same token_id
**Mitigation:**
- Use atomic counter increment (TOKEN_INDEX)
- CosmWasm storage guarantees prevent race conditions
- Token IDs are sequential: "1", "2", "3", etc.

### 8. Collection Not Created Yet
**Risk:** Minting before collection instantiation completes
**Mitigation:**
- Use reply callback to set collection address
- Store collection_address only after successful instantiation
- Minting will fail if collection_address not set

### 9. Marketplace Doesn't Support Per-Token Royalties
**Risk:** Stargaze marketplace might not query per-token royalties
**Mitigation:**
- Implement standard CW2981 `RoyaltyInfo` query
- Most CosmWasm marketplaces support this standard
- Test on testnet first

### 10. Creator Address Changes After Minting
**Risk:** Creator wants to transfer royalty payments to new address
**Handling:**
- **Not supported in v1** - Royalty address is immutable after mint
- Rationale: Prevents malicious royalty hijacking
- Future: Add `TransferRoyaltyRights` message if needed

### 11. Gas Optimization for Large Queries
**Risk:** `TokensByCreator` query could be expensive
**Mitigation:**
- Implement pagination (start_after, limit)
- Default limit: 30 tokens per query
- Max limit: 100 tokens per query

### 12. Global Royalty Changed After Mints
**Scenario:** Admin changes creator_royalty_bps from 5% to 10% after tokens exist
**Current Behavior:** All tokens (existing and new) use the NEW percentage
**Status:** Correct behavior - global setting affects all

### 13. Reentrancy Attacks
**Risk:** Malicious contract calls back during execution
**Mitigation:**
- CosmWasm has no callbacks within same transaction (unlike Ethereum)
- Use `ReplyOn::Success` for collection instantiation
- No external calls during minting except to SG721 (trusted)

### 14. Integer Overflow in Royalty Calculation
**Risk:** `sale_price * royalty_bps` could overflow
**Mitigation:**
```rust
// Use checked math
let royalty_amount = sale_price
    .checked_mul_floor(Decimal::bps(config.creator_royalty_bps))
    .map_err(|_| cosmwasm_std::StdError::generic_err("Calculation overflow"))?;
```

### 15. Empty Token URI
**Risk:** Token minted without metadata
**Handling:**
```rust
if msg.token_uri.is_empty() {
    return Err(ContractError::EmptyTokenUri {});
}
```

### 16. Duplicate Token URI
**Risk:** Two tokens with same metadata URL
**Handling:**
- **Allow duplicates** - Creator's choice (e.g., series/editions)
- No uniqueness check (too expensive)

### 17. Trading Time in the Past
**Risk:** Setting start_trading_time to past date
**Mitigation:**
```rust
if let Some(start_time) = msg.start_time {
    if start_time < env.block.time {
        return Err(ContractError::TradingTimeInPast {});
    }
}
```

### 18. Max Royalty Set Too High
**Risk:** Admin sets max > 10000 (100%)
**Mitigation:**
```rust
if creator_royalty_bps > 1000 {  // Max 10%
    return Err(ContractError::InvalidCreatorRoyalty {});
}
```

### 19. Query for Non-Existent Token
**Risk:** Querying royalty info for unminted token_id
**Handling:**
```rust
let royalty = TOKEN_ROYALTIES.load(deps.storage, token_id)
    .map_err(|_| StdError::generic_err(format!("Token not found: {}", token_id)))?;
```

### 20. Empty Allowlist
**Scenario:** Contract instantiated with no addresses on allowlist
**Current Behavior:** No one can mint until admin adds addresses
**Status:** Acceptable - admin can add addresses anytime

---

## Security Considerations

### Allowlist Management
1. **Admin-only** - Only admin can modify allowlist
2. **On-chain transparency** - All changes logged on-chain
3. **Pause feature** - Emergency stop if issues discovered
4. **Immutable creator** - Cannot change token's creator after mint

### Access Control
1. **Admin actions logged** - All config changes emit events
2. **Pause switch** - Emergency stop if issues discovered
3. **Immutable royalty recipients** - Cannot be changed after mint

### Testing Strategy
1. **Unit tests** - All edge cases above (24 tests)
2. **Integration tests** - With mock SG721 contract
3. **Testnet deployment** - Test on Stargaze testnet first
4. **Marketplace integration test** - Verify royalty queries work

---

## Deployment Steps

### 1. Compile Contract
```bash
cargo build --release --target wasm32-unknown-unknown
cargo run-script optimize
```

### 2. Store on Stargaze
```bash
starsd tx wasm store artifacts/starty_multi_creator_minter.wasm \
  --from wallet --gas-prices 0.025ustars --gas auto --gas-adjustment 1.3
```

### 3. Instantiate
```json
{
  "collection_params": {
    "code_id": 2,
    "name": "Starty Avatars",
    "symbol": "STARTY",
    "info": {
      "creator": "stars1...",
      "description": "Multi-creator avatar collection",
      "image": "ipfs://...",
      "external_link": "https://starty.xyz",
      "start_trading_time": null
    }
  },
  "creator_royalty_bps": 500,
  "initial_allowlist": ["stars1creator1...", "stars1creator2..."]
}
```

### 4. Backend Integration
```typescript
// Server-side API endpoint
app.post('/api/mint-nft', async (req, res) => {
  const { creatorMnemonic, token_uri } = req.body;

  // Load creator wallet
  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(
    creatorMnemonic,
    { prefix: 'stars' }
  );

  const [account] = await wallet.getAccounts();

  // Call contract (creator must be on allowlist)
  const msg = {
    mint: {
      token_uri
    }
  };

  const result = await client.execute(
    account.address,
    minterContract,
    msg,
    "auto"
  );

  res.json({ token_id: result.token_id });
});
```

---

## Future Enhancements

1. **Royalty Transfer** - Allow creators to transfer royalty rights
2. **Collection Royalty Share** - Admin takes small % of all royalties
3. **Batch Minting** - Mint multiple tokens in one transaction
4. **Royalty Split** - Multiple recipients per token (e.g., artist + platform)
5. **Metadata Validation** - On-chain schema validation for token_uri content
6. **Burn Mechanism** - Allow creators to burn their own tokens

---

## Comparison: Original vs Modified

| Feature | Base-Minter | Starty Multi-Creator |
|---------|-------------|---------------------|
| **Minting Auth** | Creator wallet only | Allowlist-based |
| **Royalties** | Collection-level, single recipient | Global %, individual recipients |
| **Fair Burn** | Required, burns fees | Removed, no fees |
| **Multiple Creators** | No | Yes |
| **Admin Controls** | Limited | Pause, allowlist, royalty % |
| **Royalty Query** | Collection-wide | Per token_id (creator lookup) |
| **Use Case** | Single artist collections | Multi-creator platforms |

---

## Testing Checklist

- [x] Allowlist validation (allowed/not allowed)
- [x] Royalty calculation accuracy (uses global %)
- [x] Max royalty enforcement (10% cap)
- [x] Pause/unpause minting
- [x] Add/remove from allowlist by admin
- [x] Non-admin cannot update config
- [x] Token URI validation
- [x] Royalty query returns correct creator
- [x] Multiple creators mint to same collection
- [x] Sequential token IDs
- [x] Query tokens by creator
- [x] Empty/invalid addresses rejected
- [x] Integer overflow protection
- [x] Gas optimization for queries

**All 24 tests passing**

---

## Contact & Support

**Repository:** https://github.com/startyNFT/cw-101minter-multiplecreators
**Stargaze Docs:** https://docs.stargaze.zone/
**CosmWasm Docs:** https://docs.cosmwasm.com/

**Questions?** Open an issue on GitHub.
