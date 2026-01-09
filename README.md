# Starty Multi-Creator NFT Minter

A CosmWasm smart contract for Stargaze that enables **multiple creators to mint NFTs under a single collection**, with each NFT having **individual per-token royalty settings**. Uses **secret-based authentication** instead of wallet-based access control.

## Features

✅ **Secret-Based Minting** - Anyone with the minting secret can mint (no wallet allowlist)
✅ **Per-Token Royalties** - Each NFT has individual royalty % and recipient
✅ **Multi-Creator Support** - Multiple creators can mint to the same collection
✅ **No Fair Burn** - Removed fee burning mechanism, free minting
✅ **Admin Controls** - Pause/unpause, rotate secrets, update max royalty
✅ **Edge Case Handling** - 20+ edge cases handled (see SPECIFICATION.md)
✅ **Marketplace Compatible** - Implements standard CW2981 RoyaltyInfo query

## Architecture

```
┌─────────────────────────────────────────────────┐
│  Multi-Creator Minter Contract                  │
│  ┌─────────────────────────────────────────┐  │
│  │ Secret Hash (SHA-256)                   │  │
│  │ Admin Address                           │  │
│  │ Max Royalty BPS                        │  │
│  │ Is Paused                              │  │
│  └─────────────────────────────────────────┘  │
│                                                 │
│  ┌─────────────────────────────────────────┐  │
│  │ TOKEN_ROYALTIES Map                    │  │
│  │ token_id -> {                          │  │
│  │   creator: Addr,                       │  │
│  │   royalty_bps: u64,                    │  │
│  │   token_uri: String,                   │  │
│  │   minted_at: Timestamp                 │  │
│  │ }                                       │  │
│  └─────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
                    ↓ mints to
┌─────────────────────────────────────────────────┐
│  SG721 Collection                               │
│  (Standard Stargaze NFT Collection)             │
└─────────────────────────────────────────────────┘
```

## Use Case: Starty Avatars

This contract was built for the **Starty platform** where users mint their own avatar NFTs:

1. **Starty** deploys this contract with a secret key
2. Users visit `starty.xyz` and create an avatar
3. Starty's backend calls the contract with:
   - The **secret** (stored in backend env)
   - User's **wallet address** (recipient)
   - Avatar **metadata URI** (IPFS)
   - User's **royalty %** (0-15%)
4. NFT is minted to user's wallet with their chosen royalty
5. User can sell on marketplace and receive their royalty %

**Key Benefit:** No allowlist management, just rotate the secret if compromised.

## Installation & Deployment

### Prerequisites

- Rust 1.70+
- wasm32-unknown-unknown target
- `cargo-run-script` (optional, for optimization)
- Stargaze CLI (`starsd`)

### Build

```bash
# Clone the repository
git clone https://github.com/yourorg/starty-multi-creator-minter
cd starty-multi-creator-minter/contract

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
  "minting_secret": "your-super-secret-password-change-me",
  "max_royalty_bps": 1500
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

### Mint an NFT (Anyone with Secret)

```bash
CONTRACT="stars1contractaddress..."

MINT_MSG='{
  "mint": {
    "secret": "your-super-secret-password-change-me",
    "recipient": "stars1userwalletaddress...",
    "token_uri": "ipfs://QmTokenMetadata...",
    "royalty_bps": 1000
  }
}'

starsd tx wasm execute $CONTRACT "$MINT_MSG" \
  --from wallet \
  --gas auto \
  --gas-adjustment 1.3
```

**Parameters:**
- `secret` - The minting secret (must match hash in contract)
- `recipient` - Wallet address to receive the NFT
- `token_uri` - IPFS or HTTPS URL for token metadata
- `royalty_bps` - Royalty in basis points (1000 = 10%, max 1500 = 15%)

### Admin: Update Secret

```bash
UPDATE_SECRET_MSG='{
  "update_secret": {
    "new_secret": "new-super-secret-password"
  }
}'

starsd tx wasm execute $CONTRACT "$UPDATE_SECRET_MSG" \
  --from admin-wallet
```

### Admin: Pause Minting

```bash
PAUSE_MSG='{ "set_paused": { "paused": true } }'

starsd tx wasm execute $CONTRACT "$PAUSE_MSG" \
  --from admin-wallet
```

### Admin: Update Max Royalty

```bash
UPDATE_MAX_MSG='{ "update_max_royalty": { "max_royalty_bps": 2000 } }'

starsd tx wasm execute $CONTRACT "$UPDATE_MAX_MSG" \
  --from admin-wallet
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
  "royalty_amount": "100000"
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
    "royalty_bps": 1000,
    "token_uri": "ipfs://QmTokenMetadata...",
    "minted_at": "1234567890"
  }
}
```

### Query: Verify Secret (Frontend Use)

```bash
QUERY_MSG='{ "verify_secret": { "secret": "test-secret" } }'

starsd query wasm contract-state smart $CONTRACT "$QUERY_MSG"
```

**Response:**
```json
{
  "valid": true
}
```

## Backend Integration (Node.js Example)

```typescript
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { GasPrice } from '@cosmjs/stargate';

// Server-side minting API endpoint
app.post('/api/mint-avatar', async (req, res) => {
  const { userWalletAddress, avatarIpfsUri, royaltyPercentage } = req.body;

  // Secret stored securely in environment variable
  const secret = process.env.MINTING_SECRET;
  const minterContract = process.env.MINTER_CONTRACT_ADDRESS;

  // Load admin wallet (for gas payment)
  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(
    process.env.ADMIN_MNEMONIC,
    { prefix: 'stars' }
  );

  const client = await SigningCosmWasmClient.connectWithSigner(
    'https://rpc.elgafar-1.stargaze-apis.com:443',
    wallet,
    { gasPrice: GasPrice.fromString('0.025ustars') }
  );

  const [account] = await wallet.getAccounts();

  // Mint the NFT
  const result = await client.execute(
    account.address,
    minterContract,
    {
      mint: {
        secret,
        recipient: userWalletAddress,
        token_uri: avatarIpfsUri,
        royalty_bps: royaltyPercentage * 100, // 10% = 1000 bps
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

### Secret Management

🔒 **CRITICAL:** Never expose the minting secret in client-side code!

**Best Practices:**
1. Store secret in backend `.env` file only
2. Use HTTPS for all frontend communication
3. Rotate secret regularly (monthly recommended)
4. Use server-side API endpoint for minting
5. Monitor transaction logs for suspicious activity

**Secret Visibility:**
- ⚠️ Secrets are visible in on-chain transaction data
- This is a **known limitation** of CosmWasm (all tx data is public)
- For production, consider using signature-based auth instead

### Admin Controls

- Admin can pause minting (emergency stop)
- Admin can rotate secret if compromised
- Admin can update max royalty %
- All admin actions are logged on-chain (transparent)

**Recommendation:** Use a multisig wallet as admin for production.

## Edge Cases Handled

This contract handles **20+ edge cases** including:

1. ✅ Secret brute force (SHA-256 hashing)
2. ✅ Invalid token URIs (URL validation)
3. ✅ Royalty exceeds max (validated)
4. ✅ Royalty set to 0% (allowed)
5. ✅ Invalid recipient address (validated)
6. ✅ Empty token URI (blocked)
7. ✅ Token ID collision (atomic counter)
8. ✅ Minting while paused (blocked)
9. ✅ Admin abuse (on-chain transparency)
10. ✅ Integer overflow (checked math)
11. ✅ Trading time in past (validated)
12. ✅ Max royalty > 100% (blocked)
13. ✅ Query non-existent token (error)
14. ✅ Duplicate token URIs (allowed)
15. ✅ Gas optimization (pagination)
16. ✅ Scheme validation (https/ipfs only)
17. ✅ Collection not created (reply callback)
18. ✅ Reentrancy (CosmWasm safe by design)
19. ✅ Unauthorized admin actions (checked)
20. ✅ Calculation overflow (checked_mul_floor)

See [SPECIFICATION.md](SPECIFICATION.md) for detailed edge case documentation.

## Testing

```bash
# Run unit tests
cargo test

# Run with backtrace
RUST_BACKTRACE=1 cargo test

# Run specific test
cargo test test_mint_with_secret
```

## Contract Comparison

| Feature | Base-Minter | Starty Multi-Creator |
|---------|-------------|---------------------|
| **Minting Auth** | Creator wallet only | Secret-based (anyone) |
| **Royalties** | Collection-level | Per-token |
| **Fair Burn** | Required | Removed |
| **Multiple Creators** | No | Yes |
| **Admin Controls** | Limited | Full (pause, secret, max %) |
| **Query Royalties** | Collection | Per token_id |
| **Use Case** | Single artist | Multi-creator platforms |

## Future Enhancements

- [ ] Signature-based authentication (cryptographic signatures)
- [ ] Royalty transfer (allow creators to transfer royalty rights)
- [ ] Batch minting (mint multiple tokens in one tx)
- [ ] Collection royalty share (platform takes % of all royalties)
- [ ] Whitelist mode (allowlist + secret)
- [ ] Royalty split (multiple recipients per token)
- [ ] Burn mechanism (allow creators to burn tokens)
- [ ] Metadata validation (on-chain schema validation)

## Documentation

- [SPECIFICATION.md](SPECIFICATION.md) - Full technical specification with edge cases
- [Stargaze Docs](https://docs.stargaze.zone/) - Stargaze blockchain documentation
- [CosmWasm Docs](https://docs.cosmwasm.com/) - CosmWasm smart contract framework

## License

Apache 2.0 (same as Stargaze launchpad)

## Support

- **Issues:** https://github.com/yourorg/starty-multi-creator-minter/issues
- **Stargaze Discord:** https://discord.gg/stargaze
- **Twitter:** [@StartyApp](https://twitter.com/StartyApp)

## Acknowledgments

Based on the Stargaze `base-minter` contract with significant modifications for multi-creator functionality. Thanks to the Stargaze team for the excellent NFT infrastructure.

---

**Built with ❤️ for the Starty community**
