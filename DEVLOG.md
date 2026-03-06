# Dev Log

Learning project — building a Bitcoin wallet from scratch to understand how crypto wallets work under the hood, while learning Rust. AI-assisted with Claude as a tutor, not a ghostwriter.

## 2026-03-06

**Session goal**: Understand wallet architecture before writing implementation code.

**What I learned**:
- A wallet doesn't "hold" crypto — it holds keys that prove ownership on the blockchain
- The key derivation chain: Entropy → Mnemonic → Seed → Master Key → Child Keys → Public Keys → Addresses. Each step is a one-way function (can't reverse it)
- BIP39 is the spec for mnemonic seed phrases — maps entropy to a 2048-word wordlist with a checksum
- PBKDF2 converts mnemonic → seed using a deliberately slow hash (makes brute force harder)
- You never roll your own crypto — use established crates (bip39, bitcoin)
- `::` in Rust is for accessing things inside a type/module (like `Mnemonic::generate`), `.` is for calling methods on an instance you already have

**What I built**:
- CLI menu with match statement routing user input to functions
- Stubbed out all 12 core wallet functions across 5 groups: key generation, address derivation, transactions, network, storage
- Blocked out `generate_mnemonic` with step-by-step plan before writing code

**Decisions made**:
- Start as a CLI tool, keep it simple, understand each layer before moving to the next
- Using `bip39` crate for mnemonic generation, will add `bitcoin` crate for key derivation
- Not touching transactions or network until key generation and addresses are solid

**Next session**:
- Fill in `generate_mnemonic` with real code using the bip39 crate
- Move to `mnemonic_to_seed` and `seed_to_master_key`
- Add the `bitcoin` crate to Cargo.toml for key derivation
