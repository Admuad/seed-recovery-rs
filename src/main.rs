use anyhow::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, Select};

use rustyline::error::ReadlineError;

fn get_rustyline_input(prompt: &str) -> String {
    let mut rl = rustyline::DefaultEditor::new().unwrap();
    match rl.readline(prompt) {
        Ok(line) => line,
        Err(_) => {
            std::process::exit(1);
        }
    }
}

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

/// Global state for recovery
struct RecoveryState {
    chain: ChainType,
    derivation_path: DerivationPath,
    target_address: Option<String>,
    seed_phrase: String, // stripped of '?' if known positions
    known_words: Vec<String>,
    mnemonic_length: usize,
    missing_count: usize,
    position_combinations: Vec<Vec<usize>>,
    check_balance: bool,
    rpc_url: Option<String>,
}

fn print_header() {
    println!("{}", "\n╔════════════════════════════════════════════════════════════╗".cyan().bold());
    println!("{}", "║  🔑 SEED PHRASE RECOVERY TOOL v0.4.0 🔑                     ║".cyan().bold());
    println!("{}", "║  Unknown Position Bruteforcing Support                     ║".cyan().bold());
    println!("{}", "║  Real-Time Rust Speedometer & Network-Exact Derivations    ║".cyan().bold());
    println!("{}", "╚════════════════════════════════════════════════════════════╝\n".cyan().bold());
}

fn print_line(s: &str) { println!("{}", s); }
fn print_border(s: char, repeat: usize) { println!("{}", s.to_string().repeat(repeat)); }

fn select_chain() -> Result<ChainType> {
    print_line("");
    let theme = ColorfulTheme::default();
    let items = vec![
        "🔷  EVM (Ethereum, Base, Polygon)",
        "🌊  Sui Network",
        "☀️  Solana",
        "🅰️  Aptos",
        "π   Pi Network",
        "🔺  Tron (Base58 T-prefix)",
        "🐕  Dogecoin (Base58 D-prefix)",
    ];
    let selection = Select::with_theme(&theme).with_prompt("Select Blockchain Network").items(&items).default(0).interact()?;
    match selection {
        0 => Ok(ChainType::EVM), 1 => Ok(ChainType::Sui), 2 => Ok(ChainType::Solana),
        3 => Ok(ChainType::Aptos), 4 => Ok(ChainType::PiNetwork), 5 => Ok(ChainType::Tron),
        6 => Ok(ChainType::Dogecoin), _ => Ok(ChainType::EVM),
    }
}

fn select_derivation_path(chain: &Chain) -> Result<DerivationPath> {
    print_line("");
    let theme = ColorfulTheme::default();
    let available_paths = chain.available_paths();
    if available_paths.len() == 1 {
        println!("📋 {} uses standard derivation path only", chain.name);
        return Ok(available_paths[0].clone());
    }
    let items: Vec<String> = available_paths.iter().map(|p| match p {
        DerivationPath::Standard => format!("📋 BIP44 (Standard) - {}", p.as_string(chain.coin_type)),
        DerivationPath::SegWitP2SH => format!("📋 BIP49 (SegWit P2SH) - {}", p.as_string(chain.coin_type)),
        DerivationPath::SegWitNative => format!("📋 BIP84 (SegWit Native) - {}", p.as_string(chain.coin_type)),
        DerivationPath::Custom(_) => "📋 Custom Path".to_string(),
    }).collect();
    let selection = Select::with_theme(&theme).with_prompt(format!("Select Path for {}", chain.name)).items(&items).default(0).interact()?;
    match selection {
        0 => Ok(DerivationPath::Standard), 1 => Ok(DerivationPath::SegWitP2SH),
        2 => Ok(DerivationPath::SegWitNative), 3 => {
            let custom_path = get_rustyline_input("? Enter custom path: ");
            Ok(DerivationPath::Custom(custom_path))
        },
        _ => Ok(DerivationPath::Standard),
    }
}

fn select_mnemonic_length() -> Result<usize> {
    let theme = ColorfulTheme::default();
    let items = vec!["12 words", "15 words", "18 words", "21 words", "24 words"];
    let selection = Select::with_theme(&theme).with_prompt("Select Seed Phrase Length").items(&items).default(0).interact()?;
    Ok(match selection { 0=>12, 1=>15, 2=>18, 3=>21, 4=>24, _=>12 })
}

fn get_verification_mode() -> Result<(Option<String>, bool, Option<String>)> {
    let theme = ColorfulTheme::default();
    let items = vec!["Target Address (fastest)", "Balance Check via RPC", "None"];
    let selection = Select::with_theme(&theme).with_prompt("Select Verification Mode").items(&items).default(0).interact()?;
    match selection {
        0 => {
            let target = get_rustyline_input("? Enter target address: ");
            Ok((Some(target.to_lowercase()), false, None))
        }
        1 => {
            let rpc = get_rustyline_input("? Enter RPC URL: ");
            Ok((None, true, Some(rpc)))
        }
        _ => Ok((None, false, None)),
    }
}

fn generate_combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut result = Vec::new();
    let mut current = Vec::new();
    fn combine(start: usize, n: usize, k: usize, current: &mut Vec<usize>, result: &mut Vec<Vec<usize>>) {
        if k == 0 {
            result.push(current.clone());
            return;
        }
        for i in start..=n - k {
            current.push(i);
            combine(i + 1, n, k - 1, current, result);
            current.pop();
        }
    }
    combine(0, n, k, &mut current, &mut result);
    result
}

fn get_seed_phrase(length: usize) -> Result<(Vec<String>, usize, Vec<Vec<usize>>)> {
    let prompt = format!("Paste your seed phrase (omit missing words OR use ? for known missing positions)\nExpected: {} words", length);
    
    println!("\n{}", prompt.cyan().bold());
    println!("{}", "(Use Left/Right Arrows or Backspace to edit if needed)".truecolor(150, 150, 150));
    let mut input = get_rustyline_input("> ");
    input.push('\n');
    
    let clean_input: String = input.chars().map(|c| {
        if c.is_ascii_alphabetic() || c == '?' { c.to_ascii_lowercase() } else { ' ' }
    }).collect();
    
    let words: Vec<&str> = clean_input.split_whitespace().collect();
    
    let has_question_marks = words.iter().any(|&w| w == "?");
    
    if has_question_marks {
        if words.len() != length {
            return Err(anyhow::anyhow!("If using '?', total words must exactly equal {}", length));
        }
        let missing_indexes: Vec<usize> = words.iter().enumerate().filter(|(_, &w)| w == "?").map(|(i, _)| i).collect();
        let known_words: Vec<String> = words.iter().filter(|&&w| w != "?").map(|s| s.to_string()).collect();
        if missing_indexes.len() > 3 { return Err(anyhow::anyhow!("Max 3 missing words supported.")); }
        if missing_indexes.is_empty() { return Err(anyhow::anyhow!("Use ? to mark missing words.")); }
        
        return Ok((known_words, missing_indexes.len(), vec![missing_indexes]));
    } else {
        let diff = length as isize - words.len() as isize;
        if diff <= 0 { return Err(anyhow::anyhow!("No missing words detected!")); }
        if diff > 3 { return Err(anyhow::anyhow!("Max 3 missing words supported. You are missing {}.", diff)); }
        
        let missing_count = diff as usize;
        let known_words: Vec<String> = words.iter().map(|s| s.to_string()).collect();
        let combos = generate_combinations(length, missing_count);
        println!("{} {} missing words detected! Auto-testing all {} positional combinations...", "UNKNOWN POSITION DISCOVERY:".green().bold(), missing_count, combos.len().to_string().yellow());
        
        return Ok((known_words, missing_count, combos));
    }
}

// Build candidate mnemonic from known words + permutations
fn build_candidate(known: &[String], missing_pos: &[usize], missing_words: &[String], length: usize) -> String {
    let mut candidate = Vec::with_capacity(length);
    let mut known_idx = 0;
    for i in 0..length {
        if let Some(pos_idx) = missing_pos.iter().position(|&p| p == i) {
            candidate.push(missing_words[pos_idx].clone());
        } else {
            candidate.push(known[known_idx].clone());
            known_idx += 1;
        }
    }
    candidate.join(" ")
}

fn run_recovery(state: RecoveryState) -> Result<Option<(String, String, Vec<String>, Option<String>)>> {
    let wordlist: Vec<String> = Language::English.word_list().iter().map(|s| s.to_string()).collect();
    
    let total_combinations: u64 = match state.missing_count {
        1 => 2048 * state.position_combinations.len() as u64,
        2 => 2048 * 2048 * state.position_combinations.len() as u64,
        3 => 2048 * 2048 * 2048 * state.position_combinations.len() as u64,
        _ => 0,
    };
    
    let pb = ProgressBar::new(total_combinations);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta} - {per_sec})")?
        .progress_chars("█░")
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let found = Arc::new(Mutex::new(None));
    let chain = Chain::from_type(state.chain);
    let target = state.target_address.clone();
    
    if state.missing_count == 1 {
        wordlist.par_iter().enumerate().for_each(|(_i, w1)| {
            if found.lock().unwrap().is_some() { return; }
            for pos in &state.position_combinations {
                let mnemonic = build_candidate(&state.known_words, pos, &[w1.clone()], state.mnemonic_length);
                if let Ok(mnemonic_obj) = Mnemonic::parse_in_normalized(Language::English, &mnemonic) {
                    if let Ok(address) = chain.derive_address(&mnemonic_obj, &state.derivation_path) {
                        if let Some(ref t) = target {
                            if address.to_lowercase() == t.to_lowercase() {
                                *found.lock().unwrap() = Some((mnemonic, address, vec![w1.clone()], None));
                            }
                        } else if state.check_balance && _i % 256 == 0 {
                            if let Some(ref rpc) = state.rpc_url {
                                if let Ok(Some(bal)) = rpc::check_balance(rpc, &address, chain.chain_type) {
                                    *found.lock().unwrap() = Some((mnemonic, address, vec![w1.clone()], Some(bal)));
                                }
                            }
                        }
                    }
                }
                pb.inc(1);
            }
        });
    } else if state.missing_count == 2 {
        (0..2048).into_par_iter().for_each(|i| {
            if found.lock().unwrap().is_some() { return; }
            let w1 = &wordlist[i];
            for w2 in wordlist.iter() {
                if found.lock().unwrap().is_some() { return; }
                for pos in &state.position_combinations {
                    let mnemonic = build_candidate(&state.known_words, pos, &[w1.clone(), w2.clone()], state.mnemonic_length);
                    if let Ok(mnemonic_obj) = Mnemonic::parse_in_normalized(Language::English, &mnemonic) {
                        if let Ok(address) = chain.derive_address(&mnemonic_obj, &state.derivation_path) {
                            if let Some(ref t) = target {
                                if address.to_lowercase() == t.to_lowercase() {
                                    *found.lock().unwrap() = Some((mnemonic, address, vec![w1.clone(), w2.clone()], None));
                                    break;
                                }
                            }
                        }
                    }
                    pb.inc(1);
                }
            }
        });
    } else if state.missing_count == 3 {
        // Warning: 3 missing with unknown positions is virtually impossible on CPU, but structure remains
        (0..2048).into_par_iter().for_each(|i| {
            if found.lock().unwrap().is_some() { return; }
            let w1 = &wordlist[i];
            for w2 in wordlist.iter() {
                if found.lock().unwrap().is_some() { return; }
                for w3 in wordlist.iter() {
                    for pos in &state.position_combinations {
                        let mnemonic = build_candidate(&state.known_words, pos, &[w1.clone(), w2.clone(), w3.clone()], state.mnemonic_length);
                        if let Ok(mnemonic_obj) = Mnemonic::parse_in_normalized(Language::English, &mnemonic) {
                            if let Ok(address) = chain.derive_address(&mnemonic_obj, &state.derivation_path) {
                                if let Some(ref t) = target {
                                    if address.to_lowercase() == t.to_lowercase() {
                                        *found.lock().unwrap() = Some((mnemonic, address, vec![w1.clone(), w2.clone(), w3.clone()], None));
                                        break;
                                    }
                                }
                            }
                        }
                        pb.inc(1);
                    }
                }
            }
        });
    }

    pb.finish();
    let res = found.lock().unwrap().clone();
    Ok(res)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_header();
    let chain_type = select_chain()?;
    let chain = Chain::from_type(chain_type);
    let derivation_path = select_derivation_path(&chain)?;
    let mnemonic_length = select_mnemonic_length()?;
    let (target_address, check_balance, rpc_url) = get_verification_mode()?;
    let (known_words, missing_count, position_combinations) = get_seed_phrase(mnemonic_length)?;

    let state = RecoveryState {
        chain: chain_type, derivation_path, target_address, seed_phrase: "".to_string(),
        known_words, mnemonic_length, missing_count, position_combinations, check_balance, rpc_url,
    };

    println!("\nPress Enter to begin high-speed parallel scan...");
    let theme = ColorfulTheme::default();
    let _: String = Input::with_theme(&theme).with_prompt("").allow_empty(true).interact()?;

    let start = Instant::now();
    let result = run_recovery(state);
    let elapsed = start.elapsed();

    println!("\n════════════════════════════════════════════════════════════════════");
    if let Ok(Some((mnemonic, address, words, balance))) = result {
        println!("{}", "MATCH FOUND!".green().bold());
        println!("Recovery Words: {}", words.join(" + ").yellow());
        println!("Seed: {}", mnemonic.cyan());
        println!("Address: {}", address.blue());
        if let Some(bal) = balance { println!("Balance: {}", bal.green().bold()); }
    } else {
        println!("{}", "NO MATCH FOUND.".red().bold());
    }
    println!("Time Exhausted: {:.2}s", elapsed.as_secs_f64());
    Ok(())
}
