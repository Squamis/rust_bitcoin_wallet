use std::io;

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
    // This will chain together: generate_mnemonic → mnemonic_to_seed
    // → seed_to_master_key → derive_child_key → private_to_public → public_key_to_address
    todo!()
}

fn send_transaction() {
    // This will chain: build_transaction → sign_transaction → broadcast_transaction
    todo!()
}

// === Key Generation ===

fn generate_mnemonic() {
    // Step 1: Generate a 12-word mnemonic
    // bip39 crate handles: random entropy → wordlist mapping → checksum
    // let mnemonic = Mnemonic::generate(12)

    // Step 2: Display the words so the user can write them down
    // This is the ONLY time the words are shown — there's no way to recover them later
    // println the mnemonic words

    // Step 3: Convert mnemonic → seed bytes
    // This runs PBKDF2 (a slow hash on purpose — makes brute force harder)
    // let seed = mnemonic.to_seed("optional passphrase")

    // Step 4: Return the mnemonic/seed so the next function can derive keys from it

    todo!()
}

fn mnemonic_to_seed() {
    // Convert seed phrase + optional passphrase into raw seed bytes
    todo!()
}

fn seed_to_master_key() {
    // Derive the BIP32 master private key from the seed
    todo!()
}

fn derive_child_key() {
    // Walk the derivation path (e.g. m/84'/0'/0'/0/0) to get a specific child key
    todo!()
}

// === Address Generation ===

fn private_to_public() {
    // Elliptic curve math: multiply private key by generator point
    todo!()
}

fn public_key_to_address() {
    // Hash the public key and encode it as a Bitcoin address
    todo!()
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
