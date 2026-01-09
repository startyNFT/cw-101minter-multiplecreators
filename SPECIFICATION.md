# Starty Multi-Creator NFT Minter - Technical Specification

## Overview
A CosmWasm smart contract for Stargaze that enables multiple creators to mint NFTs under a single collection, with each NFT having individual creator-specific royalty settings. Uses secret-based authentication instead of wallet-based access control.

**Base Contract:** Stargaze base-minter (1-of-1 minter)
**Target Network:** Stargaze (CosmWasm)
**Language:** Rust + CosmWasm

---

## Core Requirements

### 1. Secret-Based Minting
- **Instead of:** Creator wallet address verification
- **Use:** Shared secret (password/key) verification
- Anyone with the secret can mint NFTs to any recipient address
- Secret stored as SHA-256 hash in contract state (never store plaintext)
- Admin can update the secret hash

### 2. Per-Token Royalty Settings
- Each minted NFT has individual royalty configuration
- Creator (minter) can set royalty percentage (0-15%)
- Royalty payments go to the creator's wallet address
- Marketplace queries contract for royalty info per token_id

### 3. No Fair Burn
- Remove all Fair Burn fee mechanisms
- Minting is free (or custom fee if desired)
- No automatic community pool burning

### 4. Admin Controls
- Admin (contract instantiator) can:
  - Update minting secret hash
  - Pause/unpause minting
  - Update max royalty percentage
  - Update collection trading time

---

## State Structure

```rust
// Main configuration
pub struct Config {
    pub admin: Addr,                    // Admin address (can update config)
    pub collection_address: Addr,       // Associated SG721 collection
    pub minting_secret_hash: String,    // SHA-256 hash of minting secret
    pub max_royalty_bps: u64,          // Max royalty in basis points (1500 = 15%)
    pub is_paused: bool,               // Pause minting globally
}

// Per-token royalty information
pub struct TokenRoyalty {
    pub creator: Addr,         // Creator who minted this token
    pub royalty_bps: u64,      // Royalty in basis points (0-1500)
    pub token_uri: String,     // IPFS or HTTPS URL
    pub minted_at: Timestamp,  // When token was minted
}

// Storage maps
pub const CONFIG: Item<Config> = Item::new("config");
pub const TOKEN_INDEX: Item<u64> = Item::new("token_index");
pub const TOKEN_ROYALTIES: Map<String, TokenRoyalty> = Map::new("token_royalties");
```

---

## Message Types

### InstantiateMsg
```rust
pub struct InstantiateMsg {
    pub collection_params: CollectionParams,  // SG721 collection parameters
    pub minting_secret: String,               // Initial minting secret (will be hashed)
    pub max_royalty_bps: u64,                // Max allowed royalty (e.g., 1500 = 15%)
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
        royalty_info: None,     // We handle royalties per-token, not collection-level
    }
}
```

### ExecuteMsg
```rust
pub enum ExecuteMsg {
    // Mint a new NFT with individual royalty settings
    Mint {
        secret: String,            // Minting secret (checked against hash)
        recipient: String,         // Address to receive the NFT
        token_uri: String,         // Metadata URL (IPFS or HTTPS)
        royalty_bps: u64,         // Royalty percentage (0-max_royalty_bps)
    },

    // Admin: Update the minting secret
    UpdateSecret {
        new_secret: String,        // New secret (will be hashed and stored)
    },

    // Admin: Pause or unpause minting
    SetPaused {
        paused: bool,
    },

    // Admin: Update max royalty percentage
    UpdateMaxRoyalty {
        max_royalty_bps: u64,
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

    // Verify a secret without minting (for frontend validation)
    VerifySecret {
        secret: String,
    },
}
```

---

## Key Modifications from Base-Minter

### 1. Replace Creator Check with Secret Check
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

// Verify secret
let provided_hash = sha256(msg.secret.as_bytes());
let provided_hex = hex::encode(provided_hash);
if provided_hex != config.minting_secret_hash {
    return Err(ContractError::InvalidSecret {});
}

// Validate royalty percentage
if msg.royalty_bps > config.max_royalty_bps {
    return Err(ContractError::RoyaltyTooHigh {
        provided: msg.royalty_bps,
        max: config.max_royalty_bps,
    });
}
```

### 2. Store Per-Token Royalty Data
**After minting NFT to collection:**
```rust
// Store token royalty information
let token_royalty = TokenRoyalty {
    creator: deps.api.addr_validate(&msg.recipient)?,
    royalty_bps: msg.royalty_bps,
    token_uri: msg.token_uri.clone(),
    minted_at: env.block.time,
};
TOKEN_ROYALTIES.save(deps.storage, token_id.clone(), &token_royalty)?;
```

### 3. Implement Custom Royalty Query
```rust
pub fn query_royalty_info(
    deps: Deps,
    token_id: String,
    sale_price: Uint128,
) -> StdResult<RoyaltyInfoResponse> {
    let royalty = TOKEN_ROYALTIES.load(deps.storage, token_id)?;

    // Calculate royalty amount
    let royalty_decimal = Decimal::bps(royalty.royalty_bps);
    let royalty_amount = sale_price * royalty_decimal;

    Ok(RoyaltyInfoResponse {
        address: royalty.creator.to_string(),
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
// No payment required (or add custom logic if needed)
// Just mint the NFT directly
let mint_msg = Sg721ExecuteMsg::Mint {
    token_id: token_id.clone(),
    owner: msg.recipient.clone(),
    token_uri: Some(msg.token_uri.clone()),
    extension: None,
};
```

---

## Edge Cases & Handling

### 1. Secret Collision/Brute Force
**Risk:** Someone could try to brute force the secret hash
**Mitigation:**
- Use strong SHA-256 hashing (computationally expensive to reverse)
- Recommend long, random secrets (32+ characters)
- Admin can rotate secret anytime if compromised
- Rate limiting handled at RPC/node level (not contract level)

### 2. Secret Exposed in Transaction History
**Risk:** Secret is visible in transaction data on-chain
**Mitigation:**
- **Accept this risk** - CosmWasm transactions are public
- Secret should be rotated periodically (e.g., monthly)
- For production: Consider encrypted channels or backend signing
- Alternative: Use signature verification instead (requires cryptography module)

### 3. Invalid Token URI
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

### 4. Royalty Set to 0%
**Risk:** Creator accidentally sets 0% royalty, loses future earnings
**Handling:**
- **Allow 0%** - Creator's choice (some NFTs are meant to have no royalties)
- Frontend should warn but not block
- No minimum royalty enforced

### 5. Royalty Exceeds Maximum
**Risk:** Creator tries to set royalty > 15%
**Mitigation:**
```rust
if msg.royalty_bps > config.max_royalty_bps {
    return Err(ContractError::RoyaltyTooHigh {
        provided: msg.royalty_bps,
        max: config.max_royalty_bps,
    });
}
```

### 6. Admin Abuse (Pausing, Secret Changes)
**Risk:** Admin could pause minting or change secret maliciously
**Mitigation:**
- **Transparent operations** - All admin actions are on-chain and visible
- Consider adding governance module for admin actions (future enhancement)
- Frontend should display admin address prominently
- For production: Use multisig wallet as admin

### 7. Recipient Address Invalid
**Risk:** Minting to non-existent or invalid address
**Mitigation:**
```rust
let recipient_addr = deps.api.addr_validate(&msg.recipient)?;
```
This validates address format but cannot guarantee address exists or is accessible.

### 8. Token ID Collision
**Risk:** Two mints get same token_id
**Mitigation:**
- Use atomic counter increment (TOKEN_INDEX)
- CosmWasm storage guarantees prevent race conditions
- Token IDs are sequential: "1", "2", "3", etc.

### 9. Collection Not Created Yet
**Risk:** Minting before collection instantiation completes
**Mitigation:**
- Use reply callback to set collection address
- Store collection_address only after successful instantiation
- Minting will fail if collection_address not set (cannot query collection)

### 10. Marketplace Doesn't Support Per-Token Royalties
**Risk:** Stargaze marketplace might not query per-token royalties
**Investigation Needed:**
- Check if marketplace queries `RoyaltyInfo` per token_id
- If not, royalties won't be enforced on-chain
- May need marketplace modifications or use alternative marketplaces

**Fallback:**
- Implement standard CW2981 `RoyaltyInfo` query
- Most CosmWasm marketplaces should support this
- Test on testnet first

### 11. Creator Address Changes After Minting
**Risk:** Creator wants to transfer royalty payments to new address
**Handling:**
- **Not supported in v1** - Royalty address is immutable after mint
- Rationale: Prevents malicious royalty hijacking
- Future: Add `TransferRoyaltyRights` message if needed

### 12. Gas Optimization for Large Queries
**Risk:** `TokensByCreator` query could be expensive
**Mitigation:**
- Implement pagination (start_after, limit)
- Default limit: 30 tokens per query
- Max limit: 100 tokens per query
- Use secondary index: `MultiIndex` on creator address

### 13. Secret Transmitted Over HTTP
**Risk:** If frontend uses HTTP (not HTTPS), secret could be intercepted
**Mitigation:**
- **Require HTTPS** for production frontend
- Secret should be stored in backend environment variables
- Never expose secret in client-side JavaScript
- Mint API endpoint should be server-side only

### 14. Reentrancy Attacks
**Risk:** Malicious contract calls back during execution
**Mitigation:**
- CosmWasm has no callbacks within same transaction (unlike Ethereum)
- Use `ReplyOn::Success` for collection instantiation
- No external calls during minting except to SG721 (trusted)

### 15. Integer Overflow in Royalty Calculation
**Risk:** `sale_price * royalty_bps` could overflow
**Mitigation:**
```rust
// Use checked math
let royalty_amount = sale_price
    .checked_mul(Decimal::bps(royalty.royalty_bps))
    .map_err(|_| ContractError::CalculationOverflow)?;
```

### 16. Empty Token URI
**Risk:** Token minted without metadata
**Handling:**
- **Require non-empty string** for token_uri
```rust
if msg.token_uri.is_empty() {
    return Err(ContractError::EmptyTokenUri {});
}
```

### 17. Duplicate Token URI
**Risk:** Two tokens with same metadata URL
**Handling:**
- **Allow duplicates** - Creator's choice (e.g., series/editions)
- No uniqueness check (too expensive)
- Frontend should warn but not block

### 18. Trading Time in the Past
**Risk:** Setting start_trading_time to past date
**Mitigation:**
```rust
if let Some(start_time) = msg.start_time {
    if start_time < env.block.time {
        return Err(ContractError::TradingTimeInPast {});
    }
}
```

### 19. Max Royalty Set Too High
**Risk:** Admin sets max_royalty_bps > 10000 (100%)
**Mitigation:**
```rust
if msg.max_royalty_bps > 10000 {
    return Err(ContractError::InvalidMaxRoyalty {
        max: 10000,
    });
}
```

### 20. Query for Non-Existent Token
**Risk:** Querying royalty info for unminted token_id
**Handling:**
```rust
let royalty = TOKEN_ROYALTIES.load(deps.storage, token_id)
    .map_err(|_| ContractError::TokenNotFound { token_id })?;
```

---

## Security Considerations

### Secret Management
1. **Never log secrets** - Remove debug prints in production
2. **Rotate regularly** - Admin should update secret periodically
3. **Use environment variables** - Store secret in backend .env
4. **Backend minting only** - Frontend calls your API, API calls contract

### Access Control
1. **Admin actions logged** - All config changes emit events
2. **Pause switch** - Emergency stop if issues discovered
3. **Immutable royalties** - Cannot be changed after mint (prevents rug pulls)

### Testing Strategy
1. **Unit tests** - All edge cases above
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
  "minting_secret": "your-super-secret-password-here",
  "max_royalty_bps": 1500
}
```

### 4. Backend Integration
```typescript
// Server-side API endpoint
app.post('/api/mint-avatar', async (req, res) => {
  const { recipient, token_uri, royalty_bps } = req.body;

  // Secret stored in environment variable
  const secret = process.env.MINTING_SECRET;

  // Call contract
  const msg = {
    mint: {
      secret,
      recipient,
      token_uri,
      royalty_bps
    }
  };

  const result = await client.execute(
    minterContract,
    msg,
    "auto"
  );

  res.json({ token_id: result.token_id });
});
```

---

## Future Enhancements

1. **Signature-Based Auth** - Use cryptographic signatures instead of shared secret
2. **Royalty Transfer** - Allow creators to transfer royalty rights
3. **Collection Royalty Share** - Admin takes small % of all royalties
4. **Batch Minting** - Mint multiple tokens in one transaction
5. **Whitelist Mode** - Limit minting to specific addresses + secret
6. **Royalty Split** - Multiple recipients per token (e.g., artist + platform)
7. **Metadata Validation** - On-chain schema validation for token_uri content
8. **Burn Mechanism** - Allow creators to burn their own tokens

---

## Comparison: Original vs Modified

| Feature | Base-Minter | Starty Multi-Creator |
|---------|-------------|---------------------|
| **Minting Auth** | Creator wallet only | Secret-based (anyone with secret) |
| **Royalties** | Collection-level, single recipient | Per-token, individual creators |
| **Fair Burn** | Required, burns fees | Removed, no fees |
| **Multiple Creators** | No | Yes |
| **Admin Controls** | Limited | Pause, secret rotation, max royalty |
| **Royalty Query** | Collection-wide | Per token_id |
| **Use Case** | Single artist collections | Multi-creator platforms |

---

## Testing Checklist

- [ ] Secret validation (correct/incorrect)
- [ ] Royalty calculation accuracy
- [ ] Max royalty enforcement
- [ ] Pause/unpause minting
- [ ] Secret rotation by admin
- [ ] Non-admin cannot update config
- [ ] Token URI validation
- [ ] Royalty query returns correct creator
- [ ] Multiple creators mint to same collection
- [ ] Sequential token IDs
- [ ] Query tokens by creator
- [ ] Empty/invalid addresses rejected
- [ ] Integer overflow protection
- [ ] Gas optimization for queries
- [ ] Marketplace integration (royalties honored)

---

## Contact & Support

**Repository:** https://github.com/yourorg/starty-multi-creator-minter
**Stargaze Docs:** https://docs.stargaze.zone/
**CosmWasm Docs:** https://docs.cosmwasm.com/

**Questions?** Open an issue on GitHub.
