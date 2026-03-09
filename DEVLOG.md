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

## 2026-03-09

**Session goal**: Implement the full key generation chain from entropy to a real Bitcoin address.

**What I learned**:
- bip39 v2.2 removed `Mnemonic::generate()` — had to use `Mnemonic::from_entropy()` with manual entropy from `/dev/urandom`, which was actually better for understanding what happens under the hood
- `/dev/urandom` is the Linux kernel's CSPRNG — same entropy source most crypto libraries use internally. Cross-platform alternative is `rand` crate's `OsRng`
- BIP32 HMAC-SHA512 with key "Bitcoin seed" splits into two halves: first 32 bytes = master private key, last 32 bytes = chain code. The chain code is the "recipe" for deterministic child derivation — without it, a private key alone can only produce one public key
- Xpriv = extended private key = private key + chain code bundled together
- BIP84 path `m/84'/0'/0'/0/0`: 84'=native segwit, 0'=mainnet, 0'=first account, 0=receiving chain, 0=first address. The `'` means hardened derivation (uses private key in HMAC so leaked child can't compromise parent)
- secp256k1 curve multiplication: private_key × generator_point = public_key (one-way — can't reverse)
- Compressed public key = 33 bytes (x coordinate + 1 parity byte) vs 65 bytes uncompressed
- Hash160 = SHA256 → RIPEMD160, used for public key hashing in address generation
- p2wpkh = "pay to witness public key hash" — native segwit address type (bc1q...)
- Fingerprint = first 4 bytes of Hash160 of the public key — short wallet identifier
- `&Xpriv` in Rust = immutable reference (like a guaranteed-non-null read-only pointer). Rust enforces this at compile time — no null pointer crashes
- You should never reuse a receiving address — each payment gets a fresh one for privacy (blockchain observers can't link payments)

**What I built**:
- `generate_mnemonic` — reads /dev/urandom for 128 bits of entropy, creates 12-word BIP39 mnemonic, derives 64-byte seed via PBKDF2
- `seed_to_master_key` — BIP32 HMAC-SHA512 to get Xpriv (master private key + chain code)
- `derive_child_key` — walks BIP84 path m/84'/0'/0'/0/0 to get child Xpriv
- `private_to_public` — secp256k1 curve multiplication to get compressed public key
- `public_key_to_address` — Hash160 + bech32 encoding to get bc1q... address
- `generate_wallet` — chains all 5 functions together, prints fingerprint and address
- Added `bitcoin = "0.32.8"` crate to Cargo.toml
- Folded `mnemonic_to_seed` into `generate_mnemonic` (returns both mnemonic and seed)

**Bugs fixed**:
- Rust 1.93.0 SIGILL crash compiling serde_core — fixed by updating to 1.94.0
- `Mnemonic::generate(12)` not in bip39 v2.2 — switched to `from_entropy()`
- `word_iter()` deprecated — changed to `words()`
- `CompressedPublicKey::from(*public_key)` type mismatch — used tuple struct constructor `CompressedPublicKey(*public_key)` instead

**Decisions made**:
- Variable seed phrase word count (12/15/18/21/24) deferred as future feature
- Cross-platform entropy via `rand` crate's `OsRng` deferred as future feature
- Kept `/dev/urandom` for now — same security, good learning experience

**Next session**:
- Implement `save_wallet` and `load_wallet` — encrypt and persist the seed
- Implement `check_balance` — query a public API for UTXOs
- Then tackle `build_transaction` and `sign_transaction`
