# Rust Bitcoin Wallet

## Purpose
Learning project — Thomas is building a Bitcoin wallet to understand how crypto wallets work under the hood while learning Rust. This is about understanding every step, not shipping fast.

## Teaching approach
- Walk through every concept before writing code
- Block out functions with comments before implementing
- Thomas asks questions — answer them, don't skip ahead
- Comments should explain the "why" in plain language

## Architecture

CLI tool with 5 function groups, 12 core functions:

```
Key Generation:     generate_mnemonic, mnemonic_to_seed, seed_to_master_key, derive_child_key
Address Generation: private_to_public, public_key_to_address
Transaction:        build_transaction, sign_transaction
Network:            check_balance, broadcast_transaction
Storage:            save_wallet, load_wallet
```

Data flow: `Entropy → Mnemonic → Seed → Master Key → Child Keys → Public Keys → Addresses`

## Current state (2026-03-06)
- `src/main.rs`: CLI menu with match statement, all 12 functions stubbed with `todo!()`
- `generate_mnemonic` has 4-step plan commented out, ready to implement
- No functions are implemented yet — all return `todo!()`

## Next steps (in order)
1. Implement `generate_mnemonic` using bip39 crate
2. Implement `mnemonic_to_seed`
3. Add `bitcoin` crate to Cargo.toml
4. Implement `seed_to_master_key` and `derive_child_key`
5. Implement `private_to_public` and `public_key_to_address`
6. Then move to transactions/network/storage

## Dependencies
- `bip39 = "2.1"` — mnemonic seed phrase generation (already in Cargo.toml)
- `bitcoin` — keys, addresses, derivation (not yet added)

## Dev environment
- Rust edition 2024
- No sudo access — all tools installed to user space
- Neovim with LazyVim + Rust extra for editing
- GitHub repo: will be at Squamis/rust_bitcoin_wallet

## Rust concepts Thomas has covered
See `~/Learn_Rust/main/main.rs` for his reference file covering: types, enums, structs, match, loops, tuples, functions with return types, `::` vs `.`, `todo!()`
