# Solana veToken System

[![Solana](https://img.shields.io/badge/Solana-14F195?style=flat&logo=solana&logoColor=black)](https://solana.com)
[![Anchor](https://img.shields.io/badge/Anchor-6C3483?style=flat)](https://www.anchor-lang.com/)
[![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![React](https://img.shields.io/badge/React-61DAFB?style=flat&logo=react&logoColor=black)](https://reactjs.org/)

A production-ready vote-escrowed token (veToken) system on Solana with cumulative fee distribution, time-weighted governance, and SPL Token-2022 integration. Lock tokens to earn voting power and protocol fees.

**Program ID:** `5xjnSTgkKABxfbBz5wtfWb2ye17piZo7ad5UBFuFybzQ`

---

## Features

- **Time-Weighted Voting Power** — Lock tokens for customizable durations (1 day to 4 years) to receive veTokens
- **Cumulative Fee Distribution** — Fair, pro-rata fee sharing using MasterChef-style accounting (no dilution bugs)
- **Multiple Lock Support** — Add to existing locks with weighted-average unlock times
- **SPL Token-2022** — Built on Solana's modern token standard
- **Gas-Efficient PDAs** — Optimized account structure for low compute usage
- **Full-Stack dApp** — React frontend with Solana wallet adapter integration

---

## Quickstart

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) + [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools)
- [Anchor v0.31.1](https://www.anchor-lang.com/docs/installation)
- [Node.js 18+](https://nodejs.org/)

### Deploy Program
```bash
# Clone repo
git clone https://github.com/elmoxbt/fractional-ownership-token.git
cd fractional-ownership-token

# Install dependencies
npm install

# Build program
anchor build

# Deploy to devnet (update wallet path in Anchor.toml)
anchor deploy --provider.cluster devnet

# Initialize protocol
npm run deploy
```

### Run Frontend
```bash
cd app
npm install
npm run dev
# Open http://localhost:5173
```

---

## How It Works

### 1. Lock Tokens
Users lock base tokens for a chosen duration. The protocol mints veTokens proportional to lock time:
```
veTokens = locked_amount × (lock_duration / max_duration) × multiplier
```

### 2. Earn Protocol Fees
When fees are deposited, they're distributed pro-rata based on current veToken balances. Uses cumulative tracking to prevent dilution bugs:
```rust
cumulative_fee_per_ve_token += deposited_fees / total_ve_supply
claimable_fees = user_ve_balance × (cumulative - user_fee_debt)
```

### 3. Linear Decay
veTokens decay linearly as the unlock time approaches. Voting power and fee share decrease proportionally.

### 4. Unlock & Reclaim
After expiry, users withdraw locked tokens. veTokens are burned.

---

## Architecture

### Architectural Diagram

```
                         USERS (Wallet Adapters)
                       User A   User B   User C
                                  |
                                  v
                  FRONTEND - React + TypeScript + Vite
                         (Vercel Deployment)

              Lock Tokens | Claim Fees | Extend Lock | Unlock Tokens
                                  |
                        @coral-xyz/anchor SDK
                                  |
                                  v
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
           SOLANA PROGRAM (Anchor Framework)
    Program ID: 5xjnSTgkKABxfbBz5wtfWb2ye17piZo7ad5UBFuFybzQ
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

                     PROGRAM INSTRUCTIONS
   initialize()   lock_tokens()   unlock_tokens()   claim_fees()
   deposit_fees() extend_lock()   increase_lock()

─────────────────────────────────────────────────────────────────────

   STATE ACCOUNTS (PDAs)              TOKEN ACCOUNTS (Token-2022)

   GlobalState                        Token Vault (PDA)
   • total_locked                     Holds locked base tokens
   • total_ve_supply
   • cumulative_fee_per_ve_token      Fee Vault (PDA)
   • authority                        Holds claimable protocol fees
       |
       v                              veToken Mint
   UserLock A                         Minted to users (time-weighted)
   • locked_amount
   • unlock_time                      Base Token Mint
   • initial_ve_amount                User deposits
   • fee_debt

   UserLock B

   UserLock C

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

KEY FLOWS:

1. LOCK
   User → Transfer base tokens → Token Vault
       → Mint veTokens (time-weighted) → User veToken account
       → Update UserLock (amount, unlock_time, fee_debt)
       → Update GlobalState (total_locked, total_ve_supply)

2. DEPOSIT FEES (Authority only)
   Authority → Transfer fees → Fee Vault
            → Update GlobalState.cumulative_fee_per_ve_token

3. CLAIM
   User → Calculate: veTokens × (cumulative - fee_debt) / 1e18
       → Transfer fees: Fee Vault → User
       → Update UserLock.fee_debt = current cumulative

4. UNLOCK
   User → Burn veTokens
       → Transfer: Token Vault → User
       → Update UserLock (clear) + GlobalState (reduce totals)
```

### Program Instructions
| Instruction | Description |
|------------|-------------|
| `initialize` | Deploy protocol with base/ve mints and vaults |
| `lock_tokens` | Lock tokens, mint veTokens with time-weight |
| `increase_lock_amount` | Add tokens to existing lock (weighted avg) |
| `extend_lock_duration` | Extend unlock time, mint more veTokens |
| `unlock_tokens` | Withdraw after expiry, burn veTokens |
| `deposit_fees` | Authority deposits protocol fees |
| `claim_fees` | Users claim proportional fee share |
| `mint_tokens` | Mint test tokens (devnet only) |

### Key Accounts
- **GlobalState** — Protocol config, cumulative fee tracking, total supply
- **UserLock** — Per-user lock data, fee debt, veToken balance
- **TokenVault** — Holds locked base tokens
- **FeeVault** — Holds claimable protocol fees

---

## Tech Stack

**Smart Contract**
- Anchor Framework 0.31.1
- Rust + Solana Program Library
- SPL Token-2022
- Program Derived Addresses (PDAs)

**Frontend**
- React 18 + TypeScript
- Vite (build tooling)
- Solana Wallet Adapter
- @coral-xyz/anchor SDK

**Deployment**
- Vercel (frontend)
- Solana Devnet (program)

---

## Testing

Run the full test suite:
```bash
anchor test
```

Tests cover:
- Token locking and veToken minting
- Multiple lock additions with weighted averages
- Fee deposit and cumulative distribution
- Fee claiming with proper debt tracking
- Lock extensions and unlocking
- Math overflow protection

---

## Security Considerations

- **Math Overflow Protection** — All arithmetic uses checked operations with u128 precision
- **Authority Controls** — Fee deposits restricted to protocol authority
- **Time Validations** — Enforces min/max lock durations (1 day - 4 years)
- **Borrow Checker Safety** — No unsafe code, all mutations explicit
- **Fee Debt Tracking** — Prevents double-claiming via cumulative accounting

*Note: This is unaudited prototype code. Use at your own risk in production.*

---

## Open to Collaboration

Looking for Solana contract work in **DeFi protocols**, **RWA tokenization**, **payment systems**, or **AI agent infrastructure**. Feel free to reach out.

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

**Contact:** [@elmoxbt](https://x.com/elmoxbt) • elmoxbt@gmail.com
