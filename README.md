# Starty Multi-Creator NFT Minter

A CosmWasm smart contract for Stargaze that enables **multiple creators to mint NFTs under a single collection**, with a **global royalty percentage** applied to all tokens. Uses **allowlist-based authentication** for minting access control.

## Features

- **Allowlist-Based Minting** - Only addresses on the allowlist can mint
- **Global Royalty %** - Single royalty percentage (max 10%) applies to all creators
- **Per-Token Creator Tracking** - Each NFT records its creator for royalty payments
- **Multi-Creator Support** - Multiple creators can mint to the same collection
- **No Fair Burn** - Removed fee burning mechanism, free minting
- **Admin Controls** - Pause/unpause, manage allowlist, update royalty %
- **Edge Case Handling** - 20+ edge cases handled (see SPECIFICATION.md)
- **Marketplace Compatible** - Implements standard CW2981 RoyaltyInfo query

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  Multi-Creator Minter Contract                       │
│  ┌─────────────────────────────────────────────┐   │
│  │ Admin Address                                │   │
│  │ Creator Royalty BPS (global, max 10%)       │   │
│  │ Is Paused                                   │   │
│  └─────────────────────────────────────────────┘   │
│                                                      │
│  ┌─────────────────────────────────────────────┐   │
│  │ ALLOWLIST Map                               │   │
│  │ address -> allowed (bool)                   │   │
│  └─────────────────────────────────────────────┘   │
│                                                      │
│  ┌─────────────────────────────────────────────┐   │
│  │ TOKEN_ROYALTIES Map                         │   │
│  │ token_id -> {                               │   │
│  │   creator: Addr,    (receives royalties)    │   │
│  │   token_uri: String,                        │   │
│  │   minted_at: Timestamp                      │   │
│  │ }                                           │   │
│  └─────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
                    ↓ mints to
┌─────────────────────────────────────────────────────┐
│  SG721 Collection                                    │
│  (Standard Stargaze NFT Collection)                  │
└─────────────────────────────────────────────────────┘
```

## Use Case: Starty Avatars

This contract was built for the **Starty platform** where approved creators mint NFTs:

1. **Starty** deploys this contract and adds creator addresses to the allowlist
2. Approved creators visit `starty.xyz` and create content
3. Creator calls the contract directly (or via backend) with:
   - Their **wallet address** (must be on allowlist)
   - Content **metadata URI** (IPFS)
4. NFT is minted to creator's wallet
5. Creator can sell on marketplace and receive the global royalty %

**Key Benefit:** Admin controls who can mint. Add/remove creators as needed.

## Installation & Deployment

### Prerequisites

- Rust 1.70+
- wasm32-unknown-unknown target
- `cargo-run-script` (optional, for optimization)
- Stargaze CLI (`starsd`)

### Build

```bash
# Clone the repository
git clone https://github.com/startyNFT/cw-101minter-multiplecreators
cd cw-101minter-multiplecreators/contract

# Build optimized WASM
cargo build --release --target wasm32-unknown-unknown

# Optimize (optional, but recommended for production)
# Requires Docker
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/optimizer:0.16.0
```

The optimized WASM will be in `artifacts/starty_multi_creator_minter.wasm`.

### Deploy to Stargaze Testnet

```bash
# Store the contract
starsd tx wasm store artifacts/starty_multi_creator_minter.wasm \
  --from wallet \
  --chain-id elgafar-1 \
  --gas-prices 0.025ustars \
  --gas auto \
  --gas-adjustment 1.3 \
  --node https://rpc.elgafar-1.stargaze-apis.com:443

# Note the code_id from the response (e.g., 1234)
CODE_ID=1234

# Instantiate the contract
INIT_MSG='{
  "collection_params": {
    "code_id": 2,
    "name": "Starty Avatars",
    "symbol": "STARTY",
    "info": {
      "creator": "stars1youraddress...",
      "description": "Multi-creator avatar collection on Starty",
      "image": "ipfs://QmYourCollectionImage...",
      "external_link": "https://starty.xyz",
      "start_trading_time": null
    }
  },
  "creator_royalty_bps": 500,
  "initial_allowlist": ["stars1creator1...", "stars1creator2..."]
}'

starsd tx wasm instantiate $CODE_ID "$INIT_MSG" \
  --from wallet \
  --label "starty-multi-creator-minter" \
  --admin "stars1youraddress..." \
  --chain-id elgafar-1 \
  --gas-prices 0.025ustars \
  --gas auto \
  --gas-adjustment 1.3 \
  --node https://rpc.elgafar-1.stargaze-apis.com:443
```

## Usage

### Mint an NFT (Must be on Allowlist)

```bash
CONTRACT="stars1contractaddress..."

MINT_MSG='{
  "mint": {
    "token_uri": "ipfs://QmTokenMetadata..."
  }
}'

# Sender must be on allowlist
starsd tx wasm execute $CONTRACT "$MINT_MSG" \
  --from creator-wallet \
  --gas auto \
  --gas-adjustment 1.3
```

**Parameters:**
- `token_uri` - IPFS or HTTPS URL for token metadata

**Result:**
- NFT minted to sender's wallet
- Sender recorded as creator (receives royalties at global %)

### Admin: Add to Allowlist

```bash
ADD_MSG='{
  "add_to_allowlist": {
    "addresses": ["stars1newcreator1...", "stars1newcreator2..."]
  }
}'

starsd tx wasm execute $CONTRACT "$ADD_MSG" \
  --from admin-wallet
```

### Admin: Remove from Allowlist

```bash
REMOVE_MSG='{
  "remove_from_allowlist": {
    "addresses": ["stars1oldcreator..."]
  }
}'

starsd tx wasm execute $CONTRACT "$REMOVE_MSG" \
  --from admin-wallet
```

### Admin: Pause Minting

```bash
PAUSE_MSG='{ "set_paused": { "paused": true } }'

starsd tx wasm execute $CONTRACT "$PAUSE_MSG" \
  --from admin-wallet
```

### Admin: Update Global Royalty

```bash
UPDATE_ROYALTY_MSG='{ "update_creator_royalty": { "creator_royalty_bps": 1000 } }'

starsd tx wasm execute $CONTRACT "$UPDATE_ROYALTY_MSG" \
  --from admin-wallet
```

**Note:** Maximum royalty is 10% (1000 basis points).

### Query: Check if Address is Allowed

```bash
QUERY_MSG='{ "is_allowed": { "address": "stars1creator..." } }'

starsd query wasm contract-state smart $CONTRACT "$QUERY_MSG"
```

**Response:**
```json
{
  "allowed": true
}
```

### Query: Get Royalty Info

```bash
QUERY_MSG='{ "royalty_info": { "token_id": "1", "sale_price": "1000000" } }'

starsd query wasm contract-state smart $CONTRACT "$QUERY_MSG"
```

**Response:**
```json
{
  "address": "stars1creatorswalletaddress...",
  "royalty_amount": "50000"
}
```

### Query: Get Token Info

```bash
QUERY_MSG='{ "token_info": { "token_id": "1" } }'

starsd query wasm contract-state smart $CONTRACT "$QUERY_MSG"
```

**Response:**
```json
{
  "token_royalty": {
    "creator": "stars1creatorswalletaddress...",
    "token_uri": "ipfs://QmTokenMetadata...",
    "minted_at": "1234567890"
  }
}
```

### Query: Get Config

```bash
QUERY_MSG='{ "config": {} }'

starsd query wasm contract-state smart $CONTRACT "$QUERY_MSG"
```

**Response:**
```json
{
  "config": {
    "admin": "stars1admin...",
    "collection_address": "stars1collection...",
    "collection_code_id": 2,
    "creator_royalty_bps": 500,
    "is_paused": false
  }
}
```

## Backend Integration (Node.js Example)

```typescript
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { GasPrice } from '@cosmjs/stargate';

// Server-side minting API endpoint (for approved creators)
app.post('/api/mint-nft', async (req, res) => {
  const { creatorMnemonic, nftIpfsUri } = req.body;

  const minterContract = process.env.MINTER_CONTRACT_ADDRESS;

  // Load creator wallet
  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(
    creatorMnemonic,
    { prefix: 'stars' }
  );

  const client = await SigningCosmWasmClient.connectWithSigner(
    'https://rpc.elgafar-1.stargaze-apis.com:443',
    wallet,
    { gasPrice: GasPrice.fromString('0.025ustars') }
  );

  const [account] = await wallet.getAccounts();

  // Mint the NFT (creator must be on allowlist)
  const result = await client.execute(
    account.address,
    minterContract,
    {
      mint: {
        token_uri: nftIpfsUri,
      },
    },
    'auto'
  );

  // Extract token_id from events
  const mintEvent = result.events.find(e => e.type === 'wasm');
  const tokenId = mintEvent?.attributes.find(a => a.key === 'token_id')?.value;

  res.json({
    success: true,
    token_id: tokenId,
    tx_hash: result.transactionHash,
  });
});
```

## Security Considerations

### Allowlist Management

- **Admin-only**: Only the admin can add/remove addresses from the allowlist
- **Pause feature**: Admin can pause all minting in case of emergency
- **On-chain transparency**: All admin actions are logged on-chain

**Best Practices:**
1. Use a multisig wallet as admin for production
2. Vet creators before adding to allowlist
3. Monitor minting activity for suspicious behavior
4. Have a process for removing compromised addresses

### Royalty Configuration

- Maximum royalty capped at 10% (1000 basis points)
- Global royalty applies to all tokens equally
- Individual creators receive royalties at their wallet address
- Royalty % can be updated by admin (affects all future sales)

## Edge Cases Handled

This contract handles **20+ edge cases** including:

1. Unauthorized minting (not on allowlist)
2. Invalid token URIs (URL validation)
3. Royalty exceeds max (capped at 10%)
4. Royalty set to 0% (allowed)
5. Invalid addresses (validated)
6. Empty token URI (blocked)
7. Token ID collision (atomic counter)
8. Minting while paused (blocked)
9. Admin abuse (on-chain transparency)
10. Integer overflow (checked math)
11. Trading time in past (validated)
12. Max royalty > 100% (blocked)
13. Query non-existent token (error)
14. Duplicate token URIs (allowed)
15. Gas optimization (pagination)
16. Scheme validation (https/ipfs only)
17. Collection not created (reply callback)
18. Reentrancy (CosmWasm safe by design)
19. Unauthorized admin actions (checked)
20. Calculation overflow (checked_mul_floor)

See [SPECIFICATION.md](SPECIFICATION.md) for detailed edge case documentation.

## Testing

```bash
cd contract

# Run unit tests
cargo test

# Run with backtrace
RUST_BACKTRACE=1 cargo test

# Run specific test
cargo test test_mint_success
```

**24 tests covering:**
- Instantiation and configuration
- Minting authorization (allowlist checks)
- Token URI validation (https/ipfs only)
- Royalty calculation with global percentage
- Admin-only operations (allowlist, pause, royalty updates)
- Multi-creator scenarios
- Edge cases (empty URIs, invalid schemes, paused state)

## Contract Comparison

| Feature | Base-Minter | Starty Multi-Creator |
|---------|-------------|---------------------|
| **Minting Auth** | Creator wallet only | Allowlist-based |
| **Royalties** | Collection-level | Global % per creator |
| **Fair Burn** | Required | Removed |
| **Multiple Creators** | No | Yes |
| **Admin Controls** | Limited | Full (pause, allowlist, royalty %) |
| **Query Royalties** | Collection | Per token_id (creator lookup) |
| **Use Case** | Single artist | Multi-creator platforms |

## Messages Reference

### InstantiateMsg

```rust
{
  "collection_params": {
    "code_id": 2,
    "name": "Collection Name",
    "symbol": "SYMBOL",
    "info": {
      "creator": "stars1...",
      "description": "...",
      "image": "ipfs://...",
      "external_link": "https://...",
      "start_trading_time": null
    }
  },
  "creator_royalty_bps": 500,      // 5% global royalty
  "initial_allowlist": ["stars1...", "stars2..."]
}
```

### ExecuteMsg

| Message | Admin Only | Description |
|---------|------------|-------------|
| `Mint { token_uri }` | No (allowlist) | Mint NFT to sender |
| `AddToAllowlist { addresses }` | Yes | Add addresses to allowlist |
| `RemoveFromAllowlist { addresses }` | Yes | Remove addresses from allowlist |
| `SetPaused { paused }` | Yes | Pause/unpause minting |
| `UpdateCreatorRoyalty { creator_royalty_bps }` | Yes | Update global royalty % |
| `UpdateStartTradingTime { start_time }` | Yes | Update trading start time |

### QueryMsg

| Query | Description |
|-------|-------------|
| `Config {}` | Get contract configuration |
| `IsAllowed { address }` | Check if address is on allowlist |
| `RoyaltyInfo { token_id, sale_price }` | Get royalty amount for sale |
| `TokenInfo { token_id }` | Get token creator and metadata |
| `TokensByCreator { creator, start_after, limit }` | Get tokens by creator |

## Documentation

- [SPECIFICATION.md](SPECIFICATION.md) - Full technical specification with edge cases
- [TESTING.md](TESTING.md) - Testing procedures and coverage
- [CODE_REVIEW.md](CODE_REVIEW.md) - Code review findings
- [Stargaze Docs](https://docs.stargaze.zone/) - Stargaze blockchain documentation
- [CosmWasm Docs](https://docs.cosmwasm.com/) - CosmWasm smart contract framework

## License

Apache 2.0 (same as Stargaze launchpad)

## Support

- **Issues:** https://github.com/startyNFT/cw-101minter-multiplecreators/issues
- **Stargaze Discord:** https://discord.gg/stargaze
- **Twitter:** [@StartyNFT](https://twitter.com/StartyNFT)

## Acknowledgments

Based on the Stargaze `base-minter` contract with significant modifications for multi-creator functionality. Thanks to the Stargaze team for the excellent NFT infrastructure.
