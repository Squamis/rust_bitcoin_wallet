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

## 2026-03-11

**Session goal**: Implement save_wallet and load_wallet — encrypt and persist the seed to disk.

**What I learned**:
- AES-256-GCM = authenticated encryption — encrypts AND detects tampering. If someone modifies the file, decryption refuses instead of giving garbage
- Salt (16 random bytes) — mixed into password hash so same password produces different keys each time. Not secret, saved in the file
- Nonce (12 random bytes) — "number used once" per encryption. Reusing a nonce with the same key breaks the encryption. Also not secret
- PBKDF2 — turns a password into an encryption key, deliberately slow (600,000 rounds of SHA256) to make brute force impractical
- Turbofish `::<Type>` — tells a generic function which type to use explicitly, needed when Rust can't infer it from context (e.g., `pbkdf2_hmac::<sha2::Sha256>`)
- `.unwrap()` — extracts the value from a `Result::Ok`, crashes on `Result::Err`. Shortcut for learning, real code handles errors
- `use` — brings a name into scope so you don't have to write the full path every time
- `[0u8; 12]` — array initialization: 12 elements, all zero, type u8. Creates an empty buffer to fill with random bytes
- `#[derive(Serialize, Deserialize)]` — attribute macro that auto-generates code to convert a struct to/from YAML
- `hex::encode` — turns raw bytes into readable hex strings
- `std::fs::write` — convenience function to write a string to a file in one line (vs File::create + write_all)
- RAII in Rust — resources (like file handles) are automatically cleaned up when the variable goes out of scope, no explicit `.close()` needed

**What I built**:
- `save_wallet` — fully implemented: password input → PBKDF2 key derivation → AES-256-GCM encryption → YAML file output
- `WalletFile` struct with serde derive for YAML serialization
- `load_wallet` — blocked out with step-by-step comments (not yet implemented)
- Added 6 new crates: aes-gcm, pbkdf2, sha2, serde, serde_yaml, hex

**Decisions made**:
- YAML over JSON for wallet file format (more human-readable for a simple flat structure)
- Hex-encode all byte fields in the YAML for readability
- 600,000 PBKDF2 rounds (OWASP recommendation), configurable rounds deferred as future feature
- Save wallet.yaml in the current directory for simplicity
- Deferred `rpassword` crate for hiding password input as future feature

**What I built (continued)**:
- `load_wallet` — reads wallet.yaml, hex-decodes fields, asks password, PBKDF2 re-derives key, AES-GCM decrypts, parses mnemonic, rebuilds full wallet chain
- Wired save_wallet into generate_wallet with "Save this wallet? (y/n)" prompt
- Added wallet.yaml to .gitignore (encrypted key material shouldn't be in git)
- Full round-trip tested: generate → save → load produces same fingerprint and address

**What I built (continued)**:
- `check_balance` — queries Blockstream Esplora API for UTXOs at an address, sums values in satoshis, displays BTC conversion
- Added `reqwest` (blocking HTTP client) and `serde_json` crates
- Tested with a funded mainnet address (89 UTXOs, 0.165 BTC) and our empty generated address (0 sats)

**Next session**:
- Implement `build_transaction` and `sign_transaction`
- Implement `broadcast_transaction`
