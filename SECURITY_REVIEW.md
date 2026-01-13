# Security & Code Review Report

**Project:** Multi-Creator NFT Minter with Two-Tier Royalties
**Date:** January 2026
**Reviewer:** Claude Code

---

## Executive Summary

Overall, the codebase is well-structured and follows CosmWasm best practices. However, there are several issues ranging from **critical** to **informational** that should be addressed before mainnet deployment.

| Severity | Count |
|----------|-------|
| Critical | 1 |
| High | 2 |
| Medium | 3 |
| Low | 4 |
| Informational | 5 |

---

## Critical Issues

### 1. General Royalty Address Not Validated on Instantiation

**Location:** `contracts/collection/src/contract.rs:53`

**Description:** When instantiating the collection with a `general_royalty`, the address is stored without validation. A malformed address could be stored, causing royalty payments to fail silently.

**Current Code:**
```rust
GENERAL_ROYALTY.save(deps.storage, &msg.general_royalty)?;
```

**Impact:** Royalty payments could be directed to an invalid address, resulting in lost funds.

**Recommendation:**
```rust
if let Some(ref gr) = msg.general_royalty {
    deps.api.addr_validate(&gr.address)?;
    if gr.royalty_bps > 10000 {
        return Err(ContractError::InvalidRoyalty {});
    }
}
GENERAL_ROYALTY.save(deps.storage, &msg.general_royalty)?;
```

---

## High Severity Issues

### 2. Inconsistent Royalty Limits Between Contracts

**Location:**
- `contracts/minter/src/contract.rs:32-34` (max 1000 bps / 10%)
- `contracts/collection/src/contract.rs:412` (max 10000 bps / 100%)

**Description:** The minter enforces a maximum royalty of 10% (1000 bps), but the collection allows up to 100% (10000 bps) when updating general royalty directly.

**Impact:** If an attacker gains creator privileges on the collection, they could set royalty to 100%, effectively stealing all sale proceeds.

**Recommendation:** Add consistent validation in the collection contract:
```rust
if general_royalty.royalty_bps > 1000 {
    return Err(ContractError::InvalidRoyalty {});
}
```

### 3. Per-Token Royalty BPS Not Validated

**Location:** `contracts/collection/src/contract.rs:104-143`

**Description:** When minting a token with a `TokenExtension`, the `royalty_bps` field is not validated. A malicious minter could set `royalty_bps` to any value including > 10000.

**Impact:** Per-token royalty could exceed 100% or have unexpected values.

**Recommendation:** Add validation in `execute_mint`:
```rust
if let Some(bps) = extension.royalty_bps {
    if bps > 1000 {
        return Err(ContractError::InvalidRoyalty {});
    }
}
```

---

## Medium Severity Issues

### 4. query_approval Returns Misleading Default

**Location:** `contracts/collection/src/contract.rs:704-725`

**Description:** When an approval is not found, the function returns a default approval with `Expiration::Never {}` instead of returning an error or empty response.

**Current Code:**
```rust
let approval = token
    .approvals
    .into_iter()
    .find(...)
    .unwrap_or(cw721::Approval {
        spender: spender_addr,
        expires: Expiration::Never {},
    });
```

**Impact:** Callers may incorrectly believe an approval exists when it doesn't.

**Recommendation:** Return an error or optional response when approval not found.

### 5. No Rate Limiting on Minting

**Location:** `contracts/minter/src/contract.rs:118-201`

**Description:** There's no rate limiting mechanism. An allowlisted address could mint unlimited tokens in rapid succession.

**Impact:** Could lead to gas exhaustion attacks or spam.

**Recommendation:** Consider adding:
- Per-address mint limits
- Cooldown periods between mints
- Maximum total supply

### 6. Operator Can Approve Themselves

**Location:** `contracts/collection/src/contract.rs:306-321`

**Description:** A user can set themselves as their own operator via `ApproveAll`, which is redundant.

**Impact:** Minor gas waste, potential confusion.

**Recommendation:** Add check:
```rust
if info.sender == operator_addr {
    return Err(ContractError::CannotApproveSelf {});
}
```

---

## Low Severity Issues

### 7. Empty Token ID Edge Case

**Location:** `contracts/collection/src/contract.rs:491-511`

**Description:** If `token_id` is `Some("")` (empty string), the query will attempt to load from storage and likely fail with a confusing error.

**Recommendation:** Add early validation for empty token_id.

### 8. Token URI Scheme Case Sensitivity

**Location:** `contracts/minter/src/contract.rs:147-152`

**Description:** URI scheme check is case-sensitive. `HTTPS://` or `IPFS://` would be rejected.

**Current Code:**
```rust
if scheme != "https" && scheme != "ipfs" {
```

**Recommendation:** The `url` crate normalizes schemes to lowercase, so this is likely fine, but worth verifying.

### 9. No Collection Info Update Function

**Location:** `contracts/collection/src/contract.rs`

**Description:** Once set, collection metadata (description, image, external_link) cannot be updated.

**Impact:** Cannot fix metadata errors post-deployment.

**Recommendation:** Add `UpdateCollectionInfo` execute message.

### 10. Potential Token Count Desync

**Location:** `contracts/collection/src/contract.rs:136-137, 359-360`

**Description:** Token count is managed separately from actual tokens. While `saturating_sub` prevents underflow, if there's ever a bug, counts could desync.

**Recommendation:** Consider deriving count from actual token storage in critical operations, or add an admin function to reconcile counts.

---

## Informational

### 11. Dual Usage of "Creator" Term

**Description:** The term "creator" is used for:
1. Collection admin (who can update general royalty)
2. Token minter (who receives per-token royalty)

**Recommendation:** Consider renaming collection-level creator to "admin" or "owner" for clarity.

### 12. No Events/Attributes for Royalty Queries

**Description:** Royalty info queries return data but don't emit events for off-chain indexing.

**Recommendation:** N/A for queries, but consider adding more detailed attributes in execute messages.

### 13. Missing Schema Generation

**Description:** No `bin/schema.rs` for generating JSON schemas.

**Recommendation:** Add schema generation for API documentation.

### 14. Unused Dependencies

**Description:** `cw721-base` and `cw-ownable` are included but features may not be fully utilized.

**Recommendation:** Review and remove unused dependencies to reduce binary size.

### 15. No Migration Support

**Description:** No `migrate` entry point for contract upgrades.

**Recommendation:** Add migration support for future upgrades:
```rust
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Migration logic
}
```

---

## Test Coverage Analysis

| Area | Coverage | Notes |
|------|----------|-------|
| Basic minting | Good | Well covered |
| Transfers | Good | Including approvals |
| Royalties (per-token) | Good | Multiple scenarios |
| Royalties (general) | Good | Fallback tested |
| Two-tier royalty | Good | Both paths tested |
| Edge cases | Partial | Empty strings, limits not tested |
| Error paths | Good | Unauthorized, invalid inputs |
| Pagination | Good | Token listing |

**Missing Tests:**
- Royalty > 10000 bps handling
- Empty token_id in queries
- Self-approval scenarios
- Maximum token counts
- Invalid addresses in general royalty

---

## Recommendations Summary

### Must Fix Before Mainnet:
1. Validate general royalty address on instantiation
2. Align royalty limits between contracts (both should cap at 1000 bps)
3. Validate per-token royalty_bps in mint

### Should Fix:
4. Fix query_approval to not return misleading defaults
5. Add rate limiting or mint caps
6. Prevent self-approval

### Nice to Have:
7. Add migration support
8. Add collection info update function
9. Add schema generation
10. Clarify "creator" terminology

---

## Conclusion

The contracts implement a solid multi-creator NFT minting system with a flexible two-tier royalty mechanism. The core logic is sound, but the critical issue of unvalidated royalty addresses and the royalty limit inconsistency should be addressed before production deployment.

The test suite is comprehensive but should be expanded to cover the identified edge cases.

**Overall Assessment:** Ready for testnet with fixes required before mainnet.
