# Rust Bitcoin Wallet

## Purpose
Learning project — Thomas is building a Bitcoin wallet to understand how crypto wallets work under the hood while learning Rust. This is about understanding every step, not shipping fast.

## Teaching approach
- **Be deeply Socratic** — ask Thomas questions before giving answers. Make him reason through concepts. Don't hand him code without him understanding why each piece exists
- Ask "what do you think happens here?" and "why?" constantly
- Don't assume Thomas knows things — check understanding first, build on what he already knows
- Walk through every concept conversationally before writing a single line of code
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

## Current state (2026-03-11)
- Full key generation chain implemented and working:
  - `generate_mnemonic` — reads /dev/urandom, creates 12-word BIP39 mnemonic, derives seed
  - `seed_to_master_key` — BIP32 HMAC-SHA512 to get Xpriv (master private key + chain code)
  - `derive_child_key` — walks BIP84 path m/84'/0'/0'/0/0 to get child Xpriv
  - `private_to_public` — secp256k1 curve multiplication to get compressed public key
  - `public_key_to_address` — Hash160 + bech32 encoding to get bc1q... address
- `mnemonic_to_seed` was folded into `generate_mnemonic` (returns both mnemonic and seed)
- `save_wallet` — AES-256-GCM encryption with PBKDF2 key derivation, saves to wallet.yaml
- `load_wallet` — reads wallet.yaml, decrypts, reconstructs full wallet from mnemonic
- `generate_wallet` prompts to save after generating
- `check_balance` — queries Blockstream Esplora API for UTXOs, sums satoshis
- Remaining stubs: build_transaction, sign_transaction, broadcast_transaction

## Next steps (in order)
1. Implement `check_balance` — query a public API for UTXOs at the address
2. Implement `build_transaction` and `sign_transaction`
3. Implement `broadcast_transaction`

## Future features
- Make number of seed phrase words variable (12, 15, 18, 21, or 24) — currently hardcoded to 12
- Replace /dev/urandom with `rand` crate's `OsRng` for cross-platform entropy (same security, works on Windows/macOS)
- Configurable PBKDF2 rounds for save_wallet (currently hardcoded to 600,000)
- Hide password input with `rpassword` crate

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
