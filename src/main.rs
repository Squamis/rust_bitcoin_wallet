use std::io;
use std::io::Write as IoWrite;
use bip39::Mnemonic;
use std::fs::File;
use std::io::Read as IoRead;
use bitcoin::bip32::{DerivationPath, Xpriv};
use bitcoin::secp256k1::{PublicKey, Secp256k1};
use bitcoin::{Address, Amount, CompressedPublicKey, Network, OutPoint, Transaction, TxIn, TxOut, absolute, transaction};
use aes_gcm::{Aes256Gcm, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use serde::{Serialize, Deserialize};

// Struct to hold the encrypted wallet data for saving/loading
// Serialize = can convert struct → YAML, Deserialize = can convert YAML → struct
// Each field stores hex-encoded bytes so the YAML is human-readable
#[derive(Serialize, Deserialize)]
struct WalletFile {
    salt: String,        // 16 bytes hex-encoded
    nonce: String,       // 12 bytes hex-encoded
    ciphertext: String,  // encrypted mnemonic hex-encoded
}

// Represents one unspent transaction output (UTXO) sitting at our address
// Each UTXO is a chunk of Bitcoin we can spend — like a bill in our wallet
// Deserialize lets serde_json parse the API response directly into this struct
#[derive(Deserialize)]
struct Utxo {
    txid: String,   // which transaction created this UTXO
    vout: u32,      // which output index within that transaction
    value: u64,     // amount in satoshis
}

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
        "4" => load_existing_wallet(),
        "5" => println!("Goodbye!"),
        _ => println!("Invalid option: '{}'", input.trim()),
    }
}

// === Top-level actions (called by the menu match) ===

fn generate_wallet() {
    // Chain: generate_mnemonic → seed_to_master_key → derive_child_key → address
    let (mnemonic, seed) = generate_mnemonic();
    let master_key = seed_to_master_key(&seed);
    let child_key = derive_child_key(&master_key);
    let public_key = private_to_public(&child_key);
    let address = public_key_to_address(&public_key);

    // Print summary
    let secp = Secp256k1::new();
    println!("=== WALLET GENERATED SUCCESSFULLY ===");
    println!("  Fingerprint: {}", master_key.fingerprint(&secp));
    println!("  Receive address: {}", address);

    // Ask if the user wants to save the wallet
    print!("\nSave this wallet? (y/n): ");
    io::Write::flush(&mut io::stdout()).unwrap();
    let mut answer = String::new();
    io::stdin().read_line(&mut answer).unwrap();
    if answer.trim().to_lowercase() == "y" {
        save_wallet(&mnemonic);
    }
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

fn build_transaction(our_address: &Address) {
    // Step 1: Ask the user for the recipient address and amount to send
    print!("Enter recipient address: ");
    io::Write::flush(&mut io::stdout()).unwrap();
    let mut recipient_input = String::new();
    io::stdin().read_line(&mut recipient_input).unwrap();
    // .parse() validates the address format, checksum, and network
    // require_network ensures we don't accidentally send mainnet coins to a testnet address
    let recipient: Address = recipient_input.trim().parse::<Address<_>>().unwrap()
        .require_network(Network::Bitcoin).unwrap();

    print!("Enter amount to send (in sats): ");
    io::Write::flush(&mut io::stdout()).unwrap();
    let mut amount_input = String::new();
    io::stdin().read_line(&mut amount_input).unwrap();
    let send_amount: u64 = amount_input.trim().parse().unwrap();

    // Step 2: Fetch our UTXOs from the Esplora API (same as check_balance)
    // Query Blockstream's API for all unspent outputs at our address
    // This time we parse directly into Vec<Utxo> instead of raw JSON
    let url = format!("https://blockstream.info/api/address/{}/utxo", our_address);
    let utxos: Vec<Utxo> = reqwest::blocking::get(&url).unwrap().json().unwrap();

    // Step 3: Select which UTXOs to spend — need enough to cover amount + fee
    // Simple strategy: walk through UTXOs and keep adding until we have enough
    // TODO: smarter UTXO selection — prefer consolidating small UTXOs to avoid dust,
    //   but balance against the fact that more inputs = bigger tx = higher fee
    let fee: u64 = 500; // placeholder fee in sats — we'll calculate this properly in Step 4
    let mut selected_utxos: Vec<&Utxo> = Vec::new();
    let mut total_input: u64 = 0;

    for utxo in &utxos {
        selected_utxos.push(utxo);
        total_input += utxo.value;
        // Stop once we have enough to cover the send amount + fee
        if total_input >= send_amount + fee {
            break;
        }
    }

    // If we don't have enough funds, tell the user and bail out
    if total_input < send_amount + fee {
        println!("Not enough funds! You have {} sats but need {} sats (amount + fee)",
            total_input, send_amount + fee);
        return;
    }

    // Step 4: Calculate the transaction fee
    // Fee = transaction size in bytes × fee rate (sats per byte)
    // Size depends on number of inputs and outputs:
    //   ~11 bytes overhead + 68 bytes per input (segwit) + 31 bytes per output
    // We'll have 2 outputs: one for recipient, one for our change
    let num_inputs = selected_utxos.len() as u64;
    let num_outputs: u64 = 2; // recipient + change
    let estimated_size: u64 = 11 + (68 * num_inputs) + (31 * num_outputs);

    // Query Esplora for the current recommended fee rate (sats per byte)
    // The API returns estimates for different confirmation targets (in blocks)
    // We'll use the "6 block" target — roughly 1 hour confirmation time
    let fee_url = "https://blockstream.info/api/fee-estimates";
    let fee_estimates: serde_json::Value = reqwest::blocking::get(fee_url).unwrap().json().unwrap();
    let sat_per_byte = fee_estimates["6"].as_f64().unwrap_or(5.0); // fallback to 5 sat/byte

    let fee = (estimated_size as f64 * sat_per_byte) as u64;
    println!("\nTransaction details:");
    println!("  Inputs: {} UTXOs", num_inputs);
    println!("  Estimated size: {} bytes", estimated_size);
    println!("  Fee rate: {:.1} sat/byte", sat_per_byte);
    println!("  Fee: {} sats", fee);

    // Re-check that our selected UTXOs still cover the amount with the real fee
    // (the placeholder fee in Step 3 might have been too low)
    if total_input < send_amount + fee {
        println!("Not enough funds after fee calculation! You have {} sats but need {} sats",
            total_input, send_amount + fee);
        return;
    }

    // Step 5: Calculate change — what's left over comes back to us
    //   change = total input value - send amount - fee
    //   If we forget this step, the leftover ALL goes to the miner as fee!
    let change = total_input - send_amount - fee;
    println!("  Sending: {} sats", send_amount);
    println!("  Change back to us: {} sats", change);

    // Step 6: Build the transaction
    // Create inputs — each one points to a UTXO we're spending (by txid + vout)
    let mut inputs: Vec<TxIn> = Vec::new();
    for utxo in &selected_utxos {
        // Parse the txid string into a Txid type
        // OutPoint = txid + vout — the unique identifier for a specific UTXO
        let outpoint = OutPoint::new(utxo.txid.parse().unwrap(), utxo.vout);
        // TxIn::default() gives us an input with empty signature fields
        // We'll fill in the signature (witness data) in sign_transaction
        inputs.push(TxIn {
            previous_output: outpoint,
            ..Default::default()
        });
    }

    // Create outputs — each one locks sats to an address via a script_pubkey
    // script_pubkey = the locking script that says "only the owner of this address can spend"
    let mut outputs: Vec<TxOut> = Vec::new();

    // Output 1: send the amount to the recipient
    outputs.push(TxOut {
        value: Amount::from_sat(send_amount),
        script_pubkey: recipient.script_pubkey(),
    });

    // Output 2: send the change back to our own address
    if change > 0 {
        outputs.push(TxOut {
            value: Amount::from_sat(change),
            script_pubkey: our_address.script_pubkey(),
        });
    }

    // Assemble into a Transaction
    // version = 2 (current standard), lock_time = 0 (no timelock — can be mined immediately)
    let unsigned_tx = Transaction {
        version: transaction::Version(2),
        lock_time: absolute::LockTime::ZERO,
        input: inputs,
        output: outputs,
    };

    // Step 7: Return the unsigned transaction (signing happens in sign_transaction)
    println!("\nUnsigned transaction built successfully!");
    println!("  Inputs: {}", unsigned_tx.input.len());
    println!("  Outputs: {}", unsigned_tx.output.len());

    todo!() // TODO: return unsigned_tx once we wire up the full send flow
}

fn sign_transaction() {
    // Sign each input with the private key — proves ownership without revealing the key
    todo!()
}

// === Network ===

fn check_balance() {
    // Step 1: Ask the user for a Bitcoin address to check
    print!("Enter a Bitcoin address: ");
    io::Write::flush(&mut io::stdout()).unwrap();
    let mut address = String::new();
    io::stdin().read_line(&mut address).unwrap();
    let address = address.trim();

    // Step 2: Query Blockstream's Esplora API for UTXOs at this address
    // A UTXO = an unspent chunk of Bitcoin sitting at this address
    // The API returns a JSON array of all UTXOs (txid, output index, value in satoshis)
    let url = format!("https://blockstream.info/api/address/{}/utxo", address);
    let response = reqwest::blocking::get(&url).unwrap();
    let utxos: Vec<serde_json::Value> = response.json().unwrap();

    // Step 3: Sum up all UTXO values to get the total balance
    // Each UTXO has a "value" field in satoshis (1 BTC = 100,000,000 satoshis)
    let mut total_sats: u64 = 0;
    for utxo in &utxos {
        let value = utxo["value"].as_u64().unwrap_or(0);
        total_sats += value;
    }

    // Step 4: Display the balance
    let btc = total_sats as f64 / 100_000_000.0;
    println!("\nAddress: {}", address);
    println!("  UTXOs: {}", utxos.len());
    println!("  Balance: {} sats ({:.8} BTC)", total_sats, btc);
}

fn broadcast_transaction() {
    // Send the signed transaction to the Bitcoin network
    todo!()
}

// === Storage ===

fn save_wallet(mnemonic: &Mnemonic) {
    // Step 1: Ask the user for a password to encrypt the wallet file
    // This is NOT the seed phrase — it's a separate password just for the file on disk
    print!("Enter a password to encrypt your wallet: ");
    io::Write::flush(&mut io::stdout()).unwrap();
    let mut password = String::new();
    io::stdin().read_line(&mut password).unwrap();
    let password = password.trim();

    // Step 2: Generate a random 16-byte salt
    // The salt makes each password hash unique — two people with the same password
    // get different encryption keys. Not secret, saved alongside the ciphertext.
    let mut salt = [0u8; 16];
    let mut rng = File::open("/dev/urandom").unwrap();
    rng.read_exact(&mut salt).unwrap();

    // Step 3: Derive a 32-byte encryption key from password + salt using PBKDF2
    // PBKDF2 runs SHA256 many times (600,000 rounds) to make brute force slow
    // Same concept as mnemonic → seed, just applied to a password
    // ::<sha2::Sha256> is a "turbofish" — tells the generic function which hash to use
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(
        password.as_bytes(),
        &salt,
        600_000,
        &mut key,
    );

    // Step 4: Generate a random 12-byte nonce for AES-GCM
    // "Number used once" — must be unique per encryption with the same key
    // Not secret, saved alongside the ciphertext
    let mut nonce_bytes = [0u8; 12];
    rng.read_exact(&mut nonce_bytes).unwrap();

    // Step 5: Encrypt the mnemonic string with AES-256-GCM
    // AES-256-GCM = authenticated encryption — it both encrypts AND detects tampering
    // Inputs: key (step 3), nonce (step 4), plaintext (mnemonic words)
    // Outputs: ciphertext + auth tag (tag is appended to ciphertext automatically)
    let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, mnemonic.to_string().as_bytes()).unwrap();

    // Step 6: Save salt + nonce + ciphertext to a YAML file
    // All three are needed for decryption — salt and nonce aren't secret
    // The security comes from the password → key derivation being slow
    // hex::encode turns raw bytes into readable hex strings like "a3b1c9f2..."
    let wallet_file = WalletFile {
        salt: hex::encode(&salt),
        nonce: hex::encode(&nonce_bytes),
        ciphertext: hex::encode(&ciphertext),
    };

    let yaml = serde_yaml::to_string(&wallet_file).unwrap();
    std::fs::write("wallet.yaml", &yaml).unwrap();
    println!("\nWallet saved to wallet.yaml (encrypted)");
}

// Loads and decrypts the wallet, returns the child private key and address
// Used by both the menu ("Load wallet") and send_transaction
fn load_wallet_from_file() -> (Xpriv, Address) {
    // Step 1: Read the YAML wallet file from disk
    // Get the salt, nonce, and ciphertext back
    let yaml = std::fs::read_to_string("wallet.yaml").unwrap();
    let wallet_file: WalletFile = serde_yaml::from_str(&yaml).unwrap();

    // Step 2: Decode the hex strings back into raw bytes
    // hex::decode is the reverse of hex::encode — "a3b1c9f2" → [163, 177, 201, 242]
    // Returns Vec<u8> (dynamic size) instead of [u8; N] (fixed size)
    // because hex::decode doesn't know the length at compile time
    let salt = hex::decode(&wallet_file.salt).unwrap();
    let nonce_bytes = hex::decode(&wallet_file.nonce).unwrap();
    let ciphertext = hex::decode(&wallet_file.ciphertext).unwrap();

    // Step 3: Ask the user for their password
    print!("Enter your wallet password: ");
    io::Write::flush(&mut io::stdout()).unwrap();
    let mut password = String::new();
    io::stdin().read_line(&mut password).unwrap();
    let password = password.trim();

    // Step 4: Re-derive the encryption key from password + salt
    // Must use the exact same PBKDF2 settings (rounds, hash function)
    // If the password is right, we get the same key as when we encrypted
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(
        password.as_bytes(),
        &salt,
        600_000,
        &mut key,
    );

    // Step 5: Decrypt the ciphertext with key + nonce
    // AES-GCM will fail here if the password was wrong or the file was tampered with
    // This is the "authenticated" part — it doesn't just give you garbage, it refuses
    let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext.as_ref()).unwrap();

    // Step 6: Parse the decrypted bytes back into a Mnemonic
    // String::from_utf8 converts raw bytes → String (the 12 words separated by spaces)
    // .parse() converts the string → Mnemonic type (same as DerivationPath parse)
    let mnemonic_str = String::from_utf8(plaintext).unwrap();
    let mnemonic: Mnemonic = mnemonic_str.parse().unwrap();

    // Step 7: Rebuild the wallet — same chain as generate_wallet, just starting from saved mnemonic
    let seed = mnemonic.to_seed("");
    let master_key = seed_to_master_key(&seed);
    let child_key = derive_child_key(&master_key);
    let public_key = private_to_public(&child_key);
    let address = public_key_to_address(&public_key);

    let secp = Secp256k1::new();
    println!("=== WALLET LOADED SUCCESSFULLY ===");
    println!("  Fingerprint: {}", master_key.fingerprint(&secp));
    println!("  Receive address: {}", address);

    (child_key, address)
}

// Menu wrapper — loads wallet and prints info, doesn't need the return values
fn load_existing_wallet() {
    load_wallet_from_file();
}
