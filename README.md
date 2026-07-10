# Pinocchio Fundraiser

A raw Pinocchio port of an Anchor-based Solana crowdfunding program, built as part of the **Solana Summer School** curriculum (Day 8 challenge).

## Overview

This program reimplements a token fundraiser originally written in Anchor, using [Pinocchio](https://github.com/anza-xyz/pinocchio) — a minimal, no_std framework for writing Solana programs closer to the metal. The goal of the exercise was to preserve the exact same logic and behavior as the original Anchor program while replacing all of Anchor's macro-driven account validation, serialization, and CPI handling with manual, hand-written equivalents.

Original Anchor reference: [`ABICITYE/anchor-fundraiser`](https://github.com/ABICITYE/anchor-fundraiser)

## Instructions

| # | Instruction | Description |
|---|---|---|
| 0 | `initialize` | Creates the fundraiser PDA and sets its target amount, mint, and duration |
| 1 | `contribute` | Accepts a token contribution, capped at 10% of the target per contributor, before the deadline |
| 2 | `check_contributions` | Lets the maker withdraw all funds once the target is met, closing the fundraiser account |
| 3 | `refund` | Lets a contributor reclaim their tokens if the deadline passes without the target being met |

## State

- **Fundraiser** (90 bytes): `maker`, `mint_to_raise`, `amount_to_raise`, `current_amount`, `time_started`, `duration`, `bump`
- **Contributor** (8 bytes): `amount`

## Account Validation

Since Pinocchio provides no macro-based account checks, every instruction manually verifies:
- Signer status on relevant accounts
- PDA derivation (seeds + bump) against the account actually passed in
- Ownership and initialization state before reading/writing account data

## Building & Testing

```bash
cargo build          # compile the Rust crate
cargo build-sbf       # compile the on-chain program binary
cargo test            # run LiteSVM tests for all four instructions
```

All four instructions have passing LiteSVM tests covering their core logic, including PDA validation and CPI-based token transfers.

## Tech Stack

- [Pinocchio](https://github.com/anza-xyz/pinocchio) 0.10.2
- `pinocchio-system`, `pinocchio-token`, `pinocchio-pubkey`
- [LiteSVM](https://github.com/LiteSVM/litesvm) for testing