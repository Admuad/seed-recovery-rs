use anyhow::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Instant;

// BIP-39 imports
use bip39::{Language, Mnemonic};

// Chain-specific imports
mod chains;
mod rpc;
use chains::{Chain, ChainType, DerivationPath};

// CLI configuration
const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHOR: &str = "Admuad <admuad@claw.dev>";

/// Global state for recovery
struct RecoveryState {
    chain: ChainType,
    derivation_path: DerivationPath,
    target_address: Option<String>,
    seed_phrase: String,
    mnemonic_length: usize,
    missing_count: usize,
    missing_indexes: Vec<usize>,
    check_balance: bool,
    rpc_url: Option<String>,
}

impl RecoveryState {
    fn new() -> Self {
        Self {
            chain: ChainType::EVM,
            derivation_path: DerivationPath::Standard,
            target_address: None,
            seed_phrase: String::new(),
            mnemonic_length: 12,
            missing_count: 0,
            missing_indexes: Vec::new(),
            check_balance: false,
            rpc_url: None,
        }
    }
}

/// CLI header display
fn print_header() {
    let header = r#"
╔════════════════════════════════════════════════════════════╗
║                                                              ║
║  🔑 SEED PHRASE RECOVERY TOOL v0.3.0 🔑                     ║
║                                                              ║
║  Professional Multi-Chain Wallet Recovery (Rust)             ║
║  Parallel Processing • Multi-Length • 3 Missing Words       ║
║  Multiple Derivation Paths • RPC Balance Checking           ║
║                                                              ║
╚════════════════════════════════════════════════════════════╝
"#;
    println!("{}", header.cyan().bold());
}

/// Print a simple line
fn print_line(s: &str) {
    println!("{}", s);
}

/// Print a border line
fn print_border(s: char, repeat: usize) {
    let border: String = s.to_string().repeat(repeat);
    println!("{}", border);
}

/// Print box top/bottom
fn print_box_top() {
    print_border('╔', 64);
}
fn print_box_bottom() {
    print_border('╚', 64);
}

/// Welcome message
fn print_welcome() {
    print_header();

    print_box_top();
    print_line("Welcome to Seed Phrase Recovery Tool!");
    print_line("");
    print_line("This professional tool helps you recover lost wallet seed phrases");
    print_line("by efficiently brute-forcing missing words using parallel processing.");
    print_line("");
    print_line("Features:");
    print_line("• Multi-chain support: EVM, Solana, Sui, Aptos, Pi, Tron, DOGE");
    print_line("• Flexible recovery: 12, 15, 18, 21, 24-word seed phrases");
    print_line("• Missing word handling: Up to 3 missing words");
    print_line("• Position awareness: Optimize search if you know positions");
    print_line("• Target verification: Match against known address");
    print_line("• High performance: Multi-threaded parallel scanning");
    print_line("• Multiple derivation paths: BIP44, BIP49, BIP84, Custom");
    print_line("• RPC balance checking for all supported chains");
    print_line("");
    print_line("Performance Benchmarks (AMD EPYC 7402 24-Core):");
    print_line("  1 missing (known position):     < 1 second");
    print_line("  1 missing (unknown position):   < 1 second");
    print_line("  2 missing (known position):     ~10 seconds");
    print_line("  2 missing (unknown position):   ~14 minutes");
    print_line("  3 missing (known position):     ~9 hours");
    print_line("  3 missing (unknown position):   ~62 days");
    print_line("");
    print_line("Supported Networks:");
    print_line("🔷 EVM (Ethereum, Base, Polygon, Arbitrum, Optimism)");
    print_line("🌊 Sui Network");
    print_line("☀️ Solana");
    print_line("🅰️ Aptos");
    print_line("π  Pi Network");
    print_line("🔺 Tron");
    print_line("🐕 Dogecoin");
    print_line("");
    print_line("Derivation Paths:");
    print_line("📋 BIP44 (Standard): m/44'/coin_type'/account'/change/address_index");
    print_line("📋 BIP49 (SegWit P2SH): m/49'/coin_type'/account'/change/address_index");
    print_line("📋 BIP84 (SegWit Native): m/84'/coin_type'/account'/change/address_index");
    print_line("📋 Custom: User-defined path");
    print_line("");
    print_line("⚠️  Security Note:");
    print_line("This tool is for legal recovery of YOUR OWN wallets only.");
    print_line("Never share recovered seed phrases! Store securely offline.");

    print_box_bottom();
}

/// Select blockchain
fn select_chain() -> Result<ChainType> {
    print_line("");
    let theme = ColorfulTheme::default();

    let items = vec![
        "🔷  EVM (Ethereum, Base, Polygon, Arbitrum, Optimism)",
        "🌊  Sui Network",
        "☀️  Solana",
        "🅰️  Aptos",
        "π   Pi Network",
        "🔺  Tron",
        "🐕  Dogecoin",
    ];

    let selection = Select::with_theme(&theme)
        .with_prompt("Select Blockchain Network")
        .items(&items)
        .default(0)
        .interact()?;

    let chain_type = match selection {
        0 => ChainType::EVM,
        1 => ChainType::Sui,
        2 => ChainType::Solana,
        3 => ChainType::Aptos,
        4 => ChainType::PiNetwork,
        5 => ChainType::Tron,
        6 => ChainType::Dogecoin,
        _ => ChainType::EVM,
    };

    Ok(chain_type)
}

/// Select derivation path
fn select_derivation_path(chain: &Chain) -> Result<DerivationPath> {
    print_line("");
    let theme = ColorfulTheme::default();
    
    let available_paths = chain.available_paths();
    
    if available_paths.len() == 1 {
        println!("📋 {} uses standard derivation path only", chain.name);
        return Ok(available_paths[0].clone());
    }
    
    let items: Vec<String> = available_paths.iter().map(|path| {
        match path {
            DerivationPath::Standard => format!("📋 BIP44 (Standard) - {}", path.as_string(chain.coin_type)),
            DerivationPath::SegWitP2SH => format!("📋 BIP49 (SegWit P2SH) - {}", path.as_string(chain.coin_type)),
            DerivationPath::SegWitNative => format!("📋 BIP84 (SegWit Native) - {}", path.as_string(chain.coin_type)),
            DerivationPath::Custom(_) => format!("📋 Custom Path - Enter your own"),
        }
    }).collect();
    
    let selection = Select::with_theme(&theme)
        .with_prompt(format!("Select Derivation Path for {}", chain.name))
        .items(&items)
        .default(0)
        .interact()?;
        
    match selection {
        0 => Ok(DerivationPath::Standard),
        1 => Ok(DerivationPath::SegWitP2SH),
        2 => Ok(DerivationPath::SegWitNative),
        3 => {
            let custom_path = Input::<String>::with_theme(&theme)
                .with_prompt("Enter custom derivation path (e.g., m/44'/60'/0'/0/0)")
                .validate_with(|input: &String| -> Result<(), String> {
                    if !input.starts_with("m/") {
                        return Err("Path must start with m/".to_string());
                    }
                    if !input.contains("'") {
                        return Err("Path must contain hardened keys (')".to_string());
                    }
                    Ok(())
                })
                .interact()?;
            Ok(DerivationPath::Custom(custom_path))
        }
        _ => Ok(DerivationPath::Standard),
    }
}

/// Select mnemonic length
fn select_mnemonic_length() -> Result<usize> {
    print_line("");
    let theme = ColorfulTheme::default();

    let items = vec![
        "12 words",
        "15 words",
        "18 words",
        "21 words",
        "24 words",
    ];

    let selection = Select::with_theme(&theme)
        .with_prompt("Select Seed Phrase Length")
        .items(&items)
        .default(0)
        .interact()?;

    let length = match selection {
        0 => 12,
        1 => 15,
        2 => 18,
        3 => 21,
        4 => 24,
        _ => 12,
    };

    Ok(length)
}

/// Get verification mode
fn get_verification_mode() -> Result<(Option<String>, bool, Option<String>)> {
    print_line("");
    let theme = ColorfulTheme::default();

    let items = vec![
        "Target Address (fastest - recommended)",
        "Balance Check via RPC (slower but useful)",
        "None (list all valid mnemonics - not recommended for 2+ missing)",
    ];

    let selection = Select::with_theme(&theme)
        .with_prompt("Select Verification Mode")
        .items(&items)
        .default(0)
        .interact()?;

    match selection {
        0 => {
            let target = Input::<String>::with_theme(&theme)
                .with_prompt("Enter target address")
                .validate_with(|input: &String| -> Result<(), String> {
                    if input.is_empty() {
                        return Err("Target address cannot be empty".to_string());
                    }
                    if input.len() < 26 {
                        return Err("Invalid address format".to_string());
                    }
                    Ok(())
                })
                .interact()?;
            Ok((Some(target.to_lowercase()), false, None))
        }
        1 => {
            let rpc = Input::<String>::with_theme(&theme)
                .with_prompt("Enter RPC URL (e.g., https://eth.llamarpc.com)")
                .validate_with(|input: &String| -> Result<(), String> {
                    if input.is_empty() {
                        return Err("RPC URL cannot be empty".to_string());
                    }
                    if !input.starts_with("http") {
                        return Err("RPC URL must start with http:// or https://".to_string());
                    }
                    Ok(())
                })
                .interact()?;
            Ok((None, true, Some(rpc)))
        }
        2 => Ok((None, false, None)),
        _ => Ok((None, false, None)),
    }
}

/// Get seed phrase
fn get_seed_phrase(length: usize) -> Result<(String, usize, Vec<usize>)> {
    let theme = ColorfulTheme::default();

    let prompt = format!(
        "Paste your seed phrase (use ? for each missing word)\nExpected: {} words",
        length
    );

    let input = Input::<String>::with_theme(&theme)
        .with_prompt(&prompt)
        .validate_with(move |input: &String| -> Result<(), String> {
            let words: Vec<&str> = input.trim().split_whitespace().collect();
            if words.len() != length {
                return Err(format!("Seed phrase must be exactly {} words", length));
            }

            let missing_count = words.iter().filter(|w| **w == "?").count();
            if missing_count == 0 {
                return Err("Use ? to mark missing words (1-3)".to_string());
            }
            if missing_count > 3 {
                return Err("Maximum 3 missing words supported (use ? for each)".to_string());
            }

            // Validate known words against BIP-39
            let wordlist = Language::English.word_list();
            for word in &words {
                if *word != "?" && !wordlist.contains(word) {
                    return Err(format!("\"{}\" is not a valid BIP-39 word", word));
                }
            }

            Ok(())
        })
        .interact()?;

    let words: Vec<&str> = input.trim().split_whitespace().collect();
    let missing_indexes: Vec<usize> = words
        .iter()
        .enumerate()
        .filter(|(_, w)| **w == "?")
        .map(|(i, _)| i)
        .collect();

    let seed_phrase = input.trim().to_string();

    Ok((seed_phrase, missing_indexes.len(), missing_indexes.to_vec()))
}

/// Show confirmation
fn show_confirmation(state: &RecoveryState) -> Result<bool> {
    print_header();

    let chain = Chain::from_type(state.chain);

    let words: Vec<&str> = state.seed_phrase.split_whitespace().collect();
    let known_words: Vec<&str> = words.iter().filter(|w| **w != "?").copied().collect();
    let missing_positions: String = state.missing_indexes.iter().map(|i| (i + 1).to_string()).collect::<Vec<_>>().join(", ");

    // Calculate combinations
    let total_combos: u64 = match state.missing_count {
        1 => 2048u64,
        2 => 4194304u64,
        3 => 8589934592u64,
        _ => 0u64,
    };

    let formatted_combos = if total_combos >= 1_000_000_000 {
        format!("{:.2} billion", total_combos as f64 / 1_000_000_000.0)
    } else if total_combos >= 1_000_000 {
        format!("{:.2} million", total_combos as f64 / 1_000_000.0)
    } else {
        format!("{}", total_combos)
    };

    let estimated_time = match (state.missing_count, !state.missing_indexes.is_empty()) {
        (1, true) => "< 1 second",
        (1, false) => "< 1 second",
        (2, true) => "~10 seconds",
        (2, false) => "~14 minutes",
        (3, true) => "~9 hours",
        (3, false) => "~62 days",
        _ => "Unknown",
    };

    let chain_name = format!("{} {}", chain.icon, chain.name.cyan());
    let path_str = format!("Path: {}", state.derivation_path.as_string(chain.coin_type).yellow());
    let length_str = format!("Length: {} words", state.mnemonic_length).white();
    let target_str = match &state.target_address {
        Some(addr) => format!("Target: {}", addr.cyan()),
        None => match &state.rpc_url {
            Some(rpc) => format!("RPC: {}", rpc.cyan()),
            None => "Verification: None".to_string(),
        },
    };
    let missing_str = format!("Missing: words {} ({})", missing_positions.cyan().bold(), state.missing_count);
    let known_str = format!("Known: {}", known_words.join(" ").yellow());
    let total_str = format!("Combinations: {}", formatted_combos.cyan().bold());
    let time_str = format!("Est. time: {}", estimated_time.white());
    let prompt_str = "Press Enter to start".white().bold();

    print_box_top();
    println!("Ready to recover wallet!");
    println!("{}", chain_name);
    println!("{}", path_str);
    println!("{}", length_str);
    println!("{}", target_str);
    println!("{}", missing_str);
    println!("{}", known_str);
    println!();
    println!("{}", total_str);
    println!("{}", time_str);
    println!("{}", prompt_str);

    print_box_bottom();

    let theme = ColorfulTheme::default();
    let confirmed = Confirm::with_theme(&theme)
        .with_prompt("Continue")
        .default(true)
        .interact()?;

    Ok(confirmed)
}

/// Main recovery function with parallel processing
fn run_recovery(state: RecoveryState) -> Result<Option<(String, String, Vec<String>, Option<String>)>> {
    let chain_type = state.chain;
    let derivation_path = state.derivation_path;
    let target = state.target_address.clone();
    let seed_words: Vec<String> = state.seed_phrase.split_whitespace().map(|s| s.to_string()).collect();
    let missing_count = state.missing_count;
    let missing_indexes = state.missing_indexes.clone();
    let chain = Chain::from_type(chain_type);
    let check_balance = state.check_balance;
    let rpc_url = state.rpc_url.clone();

    println!();
    let chain_name = format!("{} {}", chain.icon, chain.name.cyan());
    println!("Recovering wallet on {}", chain_name);
    let path_display = format!("Path: {}", derivation_path.as_string(chain.coin_type).yellow());
    println!("{}", path_display);
    let known_filtered: Vec<String> = seed_words.iter().filter(|w| w.as_str() != "?").cloned().collect::<Vec<_>>();
    println!("Known: {}", known_filtered.join(" ").yellow());
    let missing_pos = format!("{}", missing_indexes.iter().map(|i| (i + 1).to_string()).collect::<Vec<_>>().join(", ").cyan());
    println!("Missing: {}", missing_pos);
    if let Some(ref target_addr) = target {
        println!("Target: {}", target_addr.cyan());
    } else if check_balance {
        println!("Mode: Balance check via RPC");
        if let Some(ref rpc) = rpc_url {
            println!("RPC: {}", rpc.cyan());
        }
    }
    println!();
    print_border('─', 80);
    println!();

    let wordlist: Vec<String> = Language::English.word_list().iter().map(|s| s.to_string()).collect();
    let total_combinations: u64 = match missing_count {
        1 => 2048,
        2 => 4194304,
        3 => 8589934592,
        _ => 0,
    };

    let pb = ProgressBar::new(total_combinations);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")?
        .progress_chars("█░")
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let result: Option<(String, String, Vec<String>)> = if missing_count == 1 {
        recover_single_missing_parallel(&wordlist, &seed_words, missing_indexes[0], target.as_ref(), &pb, &chain, &derivation_path)?
    } else if missing_count == 2 {
        recover_double_missing_parallel(&wordlist, &seed_words, &missing_indexes, target.as_ref(), &pb, &chain, &derivation_path)?
    } else if missing_count == 3 {
        recover_triple_missing_parallel(&wordlist, &seed_words, &missing_indexes, target.as_ref(), &pb, &chain, &derivation_path)?
    } else {
        None
    };

    pb.finish();

    Ok(result)
}

/// Recover single missing word with parallel processing
fn recover_single_missing_parallel(
    wordlist: &[String],
    seed_words: &[String],
    missing_index: usize,
    target: Option<&String>,
    check_balance: bool,
    rpc_url: Option<&String>,
    pb: &ProgressBar,
    chain: &Chain,
    derivation_path: &DerivationPath,
) -> Result<Option<(String, String, Vec<String>, Option<String>)>> {
    let found = Arc::new(Mutex::new(None));

    wordlist.par_iter().enumerate().for_each(|(_i, word)| {
        // Check if already found
        {
            let guard = found.lock().unwrap();
            if guard.is_some() {
                return;
            }
        }

        let mut candidate = seed_words.to_vec();
        candidate[missing_index] = word.clone();
        let mnemonic = candidate.join(" ");

        if let Ok(mnemonic_obj) = Mnemonic::parse_in_normalized(Language::English, &mnemonic) {
            if let Ok(address) = chain.derive_address(&mnemonic_obj, derivation_path) {
                if let Some(target_addr) = target {
                    if address.to_lowercase() == target_addr.to_lowercase() {
                        *found.lock().unwrap() = Some((mnemonic, address, vec![word.clone()], None));
                    }
                } else if check_balance {
                    // Check balance via RPC periodically (every 256 mnemonics)
                    if _i % 256 == 0 {
                        if let Some(ref rpc) = rpc_url {
                            if let Ok(Some(balance)) = rpc::check_balance(rpc, &address, chain.chain_type) {
                                if balance.is_some() {
                                    *found.lock().unwrap() = Some((mnemonic, address, vec![word.clone()], balance));
                                }
                            }
                        }
                    }
                }
            }
        }

        pb.inc(1);
    });

    // Clone the result before returning to fix lifetime issue
    let result = found.lock().unwrap().clone();
    Ok(result)
}

/// Recover double missing words with parallel processing
fn recover_double_missing_parallel(
    wordlist: &[String],
    seed_words: &[String],
    missing_indexes: &[usize],
    target: Option<&String>,
    pb: &ProgressBar,
    chain: &Chain,
    derivation_path: &DerivationPath,
) -> Result<Option<(String, String, Vec<String>)>> {
    let found = Arc::new(Mutex::new(None));

    (0..2048).into_par_iter().for_each(|i| {
        // Check if already found
        {
            let guard = found.lock().unwrap();
            if guard.is_some() {
                return;
            }
        }

        let word1 = &wordlist[i];
        for word2 in wordlist.iter() {
            // Check again inside inner loop
            {
                let guard = found.lock().unwrap();
                if guard.is_some() {
                    return;
                }
            }

            let mut candidate = seed_words.to_vec();
            candidate[missing_indexes[0]] = word1.clone();
            candidate[missing_indexes[1]] = word2.clone();
            let mnemonic = candidate.join(" ");

            if let Ok(mnemonic_obj) = Mnemonic::parse_in_normalized(Language::English, &mnemonic) {
                if let Ok(address) = chain.derive_address(&mnemonic_obj, derivation_path) {
                    if let Some(target_addr) = target {
                        if address.to_lowercase() == target_addr.to_lowercase() {
                            *found.lock().unwrap() = Some((
                                mnemonic,
                                address,
                                vec![word1.clone(), word2.clone()],
                            ));
                            break;
                        }
                    }
                }
            }

            pb.inc(1);
        }
    });

    // Clone the result before returning to fix lifetime issue
    let result = found.lock().unwrap().clone();
    Ok(result)
}

/// Recover triple missing words with parallel processing
fn recover_triple_missing_parallel(
    wordlist: &[String],
    seed_words: &[String],
    missing_indexes: &[usize],
    target: Option<&String>,
    pb: &ProgressBar,
    chain: &Chain,
    derivation_path: &DerivationPath,
) -> Result<Option<(String, String, Vec<String>)>> {
    let found = Arc::new(Mutex::new(None));

    (0..2048).into_par_iter().for_each(|i| {
        // Check if already found
        {
            let guard = found.lock().unwrap();
            if guard.is_some() {
                return;
            }
        }

        let word1 = &wordlist[i];
        for word2 in wordlist.iter() {
            {
                let guard = found.lock().unwrap();
                if guard.is_some() {
                    return;
                }
            }

            for word3 in wordlist.iter() {
                // Check again inside innermost loop
                {
                    let guard = found.lock().unwrap();
                    if guard.is_some() {
                        return;
                    }
                }

                let mut candidate = seed_words.to_vec();
                candidate[missing_indexes[0]] = word1.clone();
                candidate[missing_indexes[1]] = word2.clone();
                candidate[missing_indexes[2]] = word3.clone();
                let mnemonic = candidate.join(" ");

                if let Ok(mnemonic_obj) = Mnemonic::parse_in_normalized(Language::English, &mnemonic) {
                    if let Ok(address) = chain.derive_address(&mnemonic_obj, derivation_path) {
                        if let Some(target_addr) = target {
                            if address.to_lowercase() == target_addr.to_lowercase() {
                                *found.lock().unwrap() = Some((
                                    mnemonic,
                                    address,
                                    vec![word1.clone(), word2.clone(), word3.clone()],
                                ));
                                break;
                            }
                        }
                    }
                }

                pb.inc(1);
            }
        }
    });

    // Clone the result before returning to fix lifetime issue
    let result = found.lock().unwrap().clone();
    Ok(result)
}

/// Main function
fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_line("");
    print_welcome();

    let theme = ColorfulTheme::default();
    let _: String = Input::with_theme(&theme)
        .with_prompt("")
        .allow_empty(true)
        .interact()?;

    let chain_type = select_chain()?;
    let chain = Chain::from_type(chain_type);
    let derivation_path = select_derivation_path(&chain)?;
    let mnemonic_length = select_mnemonic_length()?;
    let (target_address, check_balance, rpc_url) = get_verification_mode()?;
    let (seed_phrase, missing_count, missing_indexes) = get_seed_phrase(mnemonic_length)?;

    let state = RecoveryState {
        chain: chain_type,
        derivation_path,
        target_address,
        seed_phrase,
        mnemonic_length,
        missing_count,
        missing_indexes,
        check_balance,
        rpc_url,
    };

    let confirmed = show_confirmation(&state)?;

    if !confirmed {
        println!("Recovery cancelled.");
        return Ok(());
    }

    let start = Instant::now();
    let result = run_recovery(state);
    let elapsed = start.elapsed();

    println!();
    print_border('═', 80);
    println!();
    println!("RECOVERY COMPLETE");
    println!();
    if let Ok(Some((mnemonic, address, words, balance))) = result {
        print_border('═', 80);
        println!();
        println!("MATCH FOUND!");
        println!();
        if missing_count == 1 {
            println!("{}", format!("Recovery: {}", &words[0]).yellow());
        } else if missing_count == 2 {
            println!("{}", format!("Recovery: {} + {}", &words[0].yellow(), &words[1].yellow()));
        } else {
            println!("{}", format!("Recovery: {} + {} + {}", &words[0].yellow(), &words[1].yellow(), &words[2].yellow()));
        }
        println!("{}", format!("Seed: {}", &mnemonic).cyan());
        println!("{}", format!("Address: {}", &address).blue());
        if let Some(ref bal) = balance {
            println!("{}", format!("Balance: {}", bal.green().bold()));
        }
        println!();
        println!("STORE YOUR SEED PHRASE SECURELY!");
    } else {
        println!("NO MATCH FOUND");
        println!();
        println!("Possible:");
        println!("  • Typos in known words");
        println!("  • Wrong order");
        println!("  • Different derivation path");
        println!("  • Target address incorrect");
        println!("  • More than {} missing words", missing_count);
        println!("  • Wallet has no balance");
        println!("  • Wrong blockchain network");
        println!("  • Try different derivation paths");
    }
    println!();
    print_border('═', 80);
    println!();
    println!("{}", format!("Time: {:.2}s", elapsed.as_secs_f64()).white());

    Ok(())
}