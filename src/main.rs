use std::io;
use bip39::Mnemonic;
use std::fs::File;
use std::io::Read as IoRead;
use bitcoin::bip32::{DerivationPath, Xpriv};
use bitcoin::secp256k1::{PublicKey, Secp256k1};
use bitcoin::{Address, CompressedPublicKey, Network};

fn main() {
    println!("=== Rust Bitcoin Wallet ===\n");
    println!("1. Generate new wallet");
    println!("2. Check balance");
    println!("3. Send transaction");
    println!("4. Load wallet");
    println!("5. Exit\n");

    print!("Choose an option: ");
    // flush stdout so the prompt appears before we wait for input
    // (print! doesn't auto-flush like println! does)
    io::Write::flush(&mut io::stdout()).unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    // trim() removes the newline character from hitting Enter
    match input.trim() {
        "1" => generate_wallet(),
        "2" => check_balance(),
        "3" => send_transaction(),
        "4" => load_wallet(),
        "5" => println!("Goodbye!"),
        _ => println!("Invalid option: '{}'", input.trim()),
    }
}

// === Top-level actions (called by the menu match) ===

fn generate_wallet() {
    // Chain: generate_mnemonic → seed_to_master_key → derive_child_key → address
    let (_mnemonic, seed) = generate_mnemonic();
    let master_key = seed_to_master_key(&seed);
    let child_key = derive_child_key(&master_key);
    let public_key = private_to_public(&child_key);
    let address = public_key_to_address(&public_key);

    // Print summary
    let secp = Secp256k1::new();
    println!("=== WALLET GENERATED SUCCESSFULLY ===");
    println!("  Fingerprint: {}", master_key.fingerprint(&secp));
    println!("  Receive address: {}", address);
    println!("\nYou could send Bitcoin to this address right now.");
    println!("Next to implement: transactions, balance checking, storage.");
}

fn send_transaction() {
    // This will chain: build_transaction → sign_transaction → broadcast_transaction
    todo!()
}

// === Key Generation ===

// Returns both the mnemonic (for backup) and the 64-byte seed (for key derivation)
fn generate_mnemonic() -> (Mnemonic, [u8; 64]) {
    // Step 1: Generate a 12-word mnemonic
    // 12 words = 128 bits of entropy (16 bytes). More words = more entropy:
    //   12 words = 128 bits, 15 = 160, 18 = 192, 21 = 224, 24 = 256
    // We read random bytes from /dev/urandom — the OS kernel's cryptographic RNG
    // Then bip39 maps that entropy → wordlist words + checksum
    let mut entropy = [0u8; 16]; // 16 bytes = 128 bits = 12 words
    let mut rng = File::open("/dev/urandom").unwrap();
    rng.read_exact(&mut entropy).unwrap();
    let mnemonic = Mnemonic::from_entropy(&entropy).unwrap();

    // Step 2: Display the words so the user can write them down
    // This is the ONLY time the words are shown — there's no way to recover them later
    // In a real wallet you'd make the user confirm they wrote them down
    println!("\n=== YOUR SEED PHRASE (WRITE THIS DOWN!) ===\n");
    for (i, word) in mnemonic.words().enumerate() {
        println!("  {}. {}", i + 1, word);
    }
    println!("\n=== DO NOT SHARE THIS WITH ANYONE ===\n");

    // Step 3: Convert mnemonic → seed bytes
    // This runs PBKDF2 (a slow hash on purpose — makes brute force harder)
    // Empty string means no extra passphrase — adding one would give a completely different seed
    let seed = mnemonic.to_seed("");
    println!("Seed ({} bytes): {:x?}", seed.len(), &seed[..8]);
    println!("(showing first 8 bytes only — the full seed is 64 bytes)\n");

    // Step 4: Return the mnemonic and seed so the next function can derive keys
    (mnemonic, seed)
}


fn seed_to_master_key(seed: &[u8; 64]) -> Xpriv {
    // BIP32: Take the 64-byte seed and run HMAC-SHA512 with key "Bitcoin seed"
    // First 32 bytes → master private key, last 32 bytes → chain code
    // The chain code is used later for deriving child keys deterministically
    // Network::Bitcoin = mainnet (use Network::Testnet for test coins)
    let master_key = Xpriv::new_master(Network::Bitcoin, seed).unwrap();

    println!("Master key derived successfully (BIP32 root)");
    println!("  Depth: {} (root level)", master_key.depth);
    println!("  Network: mainnet");
    // Never print the actual private key bytes in a real wallet!
    println!("  (private key bytes hidden for security)\n");

    master_key
}

fn derive_child_key(master_key: &Xpriv) -> Xpriv {
    // BIP84 derivation path: m/84'/0'/0'/0/0
    // Each level in the path means something:
    //   84'  = BIP84 (native segwit addresses starting with bc1)
    //   0'   = Bitcoin mainnet (1 = testnet)
    //   0'   = first account (you could have multiple accounts)
    //   0    = external chain (0 = receiving addresses, 1 = change addresses)
    //   0    = first address index (increment this for each new address)
    // The ' (apostrophe) means "hardened" derivation — uses the private key
    // instead of public key in the HMAC, so a leaked child can't compromise the parent

    // Step 1: Build the derivation path from the string "m/84'/0'/0'/0/0"
    // .parse() converts the string into a DerivationPath — a list of child indexes
    // Each segment becomes a ChildNumber (either Normal or Hardened)
    let path: DerivationPath = "m/84'/0'/0'/0/0".parse().unwrap();

    // Step 2: Derive the child key by walking each level of the path
    // At each level: HMAC-SHA512(chain_code, parent_key + index) → new key + new chain code
    // Secp256k1::new() creates the elliptic curve context needed for key math
    let secp = bitcoin::secp256k1::Secp256k1::new();
    let child_key = master_key.derive_priv(&secp, &path).unwrap();

    // Step 3: Print info and return the child Xpriv
    println!("Child key derived at path: m/84'/0'/0'/0/0");
    println!("  Depth: {} (5 levels deep from master)", child_key.depth);
    println!("  This is your first receiving address key\n");

    child_key
}

// === Address Generation ===

fn private_to_public(child_key: &Xpriv) -> PublicKey {
    // Elliptic curve math: private_key × generator_point = public_key
    // This is a one-way operation — you can't reverse it to get the private key
    // The Secp256k1 context holds precomputed tables that speed up the curve math
    let secp = Secp256k1::new();

    // .private_key is the raw 32-byte secret key inside the Xpriv
    // PublicKey::from_secret_key does the curve multiplication
    let public_key = PublicKey::from_secret_key(&secp, &child_key.private_key);

    // A public key is a point on the curve (x, y coordinates)
    // "Compressed" format = 33 bytes (just the x coordinate + 1 byte to indicate which y)
    // "Uncompressed" = 65 bytes (both x and y). Bitcoin uses compressed.
    println!("Public key derived (compressed, 33 bytes)");
    println!("  {}\n", public_key);

    public_key
}

fn public_key_to_address(public_key: &PublicKey) -> Address {
    // BIP84 = native segwit = "witness version 0" = addresses starting with bc1q
    // The steps under the hood:
    //   1. SHA256 the compressed public key (33 bytes → 32 bytes)
    //   2. RIPEMD160 that result (32 bytes → 20 bytes) — this is the "pubkey hash"
    //   3. Encode as bech32 with the "bc" prefix (mainnet) and witness version 0
    // The result is a bc1q... address — shorter and cheaper in transaction fees
    // than older address formats (1... or 3...)

    // CompressedPublicKey wraps the secp256k1 public key for use with bitcoin address types
    let compressed = CompressedPublicKey(*public_key);

    // p2wpkh = "pay to witness public key hash" — the native segwit address type
    let address = Address::p2wpkh(&compressed, Network::Bitcoin);

    println!("Bitcoin address (BIP84 native segwit):");
    println!("  {}\n", address);

    address
}

// === Transaction ===

fn build_transaction() {
    // Construct a transaction: which UTXOs to spend, where to send, how much fee
    todo!()
}

fn sign_transaction() {
    // Sign each input with the private key — proves ownership without revealing the key
    todo!()
}

// === Network ===

fn check_balance() {
    // Query the blockchain for unspent outputs (UTXOs) at this address
    todo!()
}

fn broadcast_transaction() {
    // Send the signed transaction to the Bitcoin network
    todo!()
}

// === Storage ===

fn save_wallet() {
    // Encrypt and save the seed phrase to disk
    todo!()
}

fn load_wallet() {
    // Decrypt and load the seed phrase from disk
    todo!()
}
