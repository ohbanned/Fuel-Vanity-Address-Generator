// Import required modules - remove unused imports
use clap::{Parser, Subcommand};
use fuel_crypto::{SecretKey, PublicKey};
use indicatif::{ProgressBar, ProgressStyle};
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Sha256, Digest};
use std::{
    io::{self, Write},
    str::FromStr,
    sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}},
    time::Instant,
};

#[derive(Subcommand, Debug, Clone)]
enum Command {
    /// Look for addresses with a specific prefix
    Prefix {
        /// The pattern to search for at the beginning of addresses
        pattern: String,
    },
    /// Look for addresses with a specific suffix
    Suffix {
        /// The pattern to search for at the end of addresses
        pattern: String,
    },
    /// Look for addresses containing a specific pattern anywhere
    Contains {
        /// The pattern to search for anywhere in addresses
        pattern: String,
    },
    /// Exit the program
    Exit,
    /// Show information about commands
    Info,
    /// Run in interactive mode
    Interactive,
}

#[derive(Parser, Debug)]
#[command(
    name = "fuel-vanity-generator",
    author = "Fuel Vanity Generator",
    about = "Generate Fuel wallet addresses with custom patterns",
    version = "0.1.0"
)]
struct Args {
    /// Run in interactive mode (default), or execute a single command
    #[arg(short, long)]
    interactive: bool,

    #[command(subcommand)]
    command: Option<Command>,

    /// Number of threads to use (default: all available cores)
    #[arg(short, long, default_value_t = num_cpus::get())]
    threads: usize,

    /// Case sensitive pattern matching
    #[arg(short, long, default_value_t = false)]
    case_sensitive: bool,
}

#[derive(Debug, Copy, Clone)]
enum Position {
    /// Look for pattern at the beginning of the address
    Prefix,
    /// Look for pattern at the end of the address
    Suffix,
    /// Look for pattern anywhere in the address
    Anywhere,
}

fn matches_pattern(address: &str, pattern: &str, position: &str, case_sensitive: bool) -> bool {
    // Remove the "0x" prefix if it exists
    let address = if address.starts_with("0x") {
        &address[2..]
    } else {
        address
    };

    if !case_sensitive {
        let address = address.to_lowercase();
        let pattern = pattern.to_lowercase();
        
        match position {
            "prefix" => address.starts_with(&pattern),
            "suffix" => address.ends_with(&pattern),
            "anywhere" => address.contains(&pattern),
            _ => false,
        }
    } else {
        match position {
            "prefix" => address.starts_with(pattern),
            "suffix" => address.ends_with(pattern),
            "anywhere" => address.contains(pattern),
            _ => false,
        }
    }
}

// Generate a random private key
fn generate_random_private_key() -> String {
    let mut key_data = [0u8; 32];
    OsRng.fill_bytes(&mut key_data);
    hex::encode(key_data)
}

// Function to convert an address to mixed case for better visual representation
// when case-sensitive matching is enabled
fn convert_to_mixed_case(address: &str) -> String {
    if !address.starts_with("0x") {
        return address.to_string();
    }
    
    let address_part = &address[2..]; // Skip the 0x prefix
    let mut result = String::from("0x");
    
    for (i, c) in address_part.chars().enumerate() {
        if c >= 'a' && c <= 'f' {
            // Convert some of the hex letters to uppercase based on position
            if i % 3 == 0 {
                result.push((c as u8 - b'a' + b'A') as char);
            } else {
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }
    
    result
}

// Generate an address from a private key with case-sensitive option
fn get_address_from_private_key_case_sensitive(private_key: &str, preserve_case: bool) -> std::result::Result<String, Box<dyn std::error::Error>> {
    // Ensure the private key is padded to 64 characters
    let padded_key = match private_key.len() {
        64 => private_key.to_string(),
        _ => format!("{:0>64}", private_key)
    };
    
    // Convert to a Fuel SecretKey
    let secret_key = SecretKey::from_str(&padded_key)?;
    
    // Get the public key from the secret key
    let public_key = PublicKey::from(&secret_key);
    
    // In Fuel, the address is derived as the SHA-256 hash of the public key
    let mut hasher = Sha256::new();
    hasher.update(public_key.as_ref());
    let address_bytes = hasher.finalize();
    
    // Format with 0x prefix
    let address_str = if preserve_case {
        // Use a mixed-case encoding for case-sensitive display
        format!("0x{}", encode_mixed_case(&address_bytes))
    } else {
        // Use regular lowercase hex
        format!("0x{}", hex::encode(address_bytes))
    };
    
    Ok(address_str)
}

// Get address from private key (backward compatibility)
fn get_address_from_private_key(private_key: &str) -> std::result::Result<String, Box<dyn std::error::Error>> {
    get_address_from_private_key_case_sensitive(private_key, false)
}

// Function to encode bytes with mixed-case for better visual diversity
fn encode_mixed_case(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len() * 2);
    for &byte in bytes.iter() {
        // For every other byte, use uppercase for one of the two hex characters
        let high = byte >> 4;
        let low = byte & 0xF;
        
        // Convert to hex chars with some randomness in the casing
        if high < 10 {
            result.push((b'0' + high) as char);
        } else {
            // Use uppercase for some of the alpha chars
            if byte % 3 == 0 {
                result.push((b'A' + (high - 10)) as char);
            } else {
                result.push((b'a' + (high - 10)) as char);
            }
        }
        
        if low < 10 {
            result.push((b'0' + low) as char);
        } else {
            // Use uppercase for some of the alpha chars
            if byte % 2 == 0 {
                result.push((b'A' + (low - 10)) as char);
            } else {
                result.push((b'a' + (low - 10)) as char);
            }
        }
    }
    result
}

// Enhanced search function with beautiful UI
async fn search_vanity_address(
    pattern: String, 
    position: String, 
    case_sensitive: bool
) -> Vec<(String, String)> {
    // Create a progress bar with beautiful formatting
    let progress = Arc::new(Mutex::new(ProgressBar::new(100)));
    {
        let progress_bar = progress.lock().unwrap();
        progress_bar.set_style(ProgressStyle::default_bar()
            .template("\r\x1b[2K\x1b[1;32mSearched:\x1b[0m {pos} | \x1b[1;32mFound:\x1b[0m {msg} | \x1b[1;35mRate:\x1b[0m {per_sec}/s")
            .unwrap());
    }
    
    // Create a results vector to store (address, private_key) pairs
    let results: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
    
    // Set up thread count
    let _num_threads = num_cpus::get();
    
    // Display beautiful configuration header with fixed width
    println!("\n\x1b[1;32m╔════════════════════════════════════════════════════╗");
    println!("║           VANITY ADDRESS SEARCH                 ║");
    println!("╚════════════════════════════════════════════════════╝");
    println!("╔════════════════════════════════════════════════════╗");
    println!("║ \x1b[1;33mPattern:\x1b[0m {:<40} ║", pattern);
    println!("║ \x1b[1;33mPosition:\x1b[0m {:<40} ║", position);
    println!("║ \x1b[1;33mCase Sensitive:\x1b[0m {:<32} ║", case_sensitive);
    println!("╚════════════════════════════════════════════════════╝");
    println!("\x1b[1;33m⚠️  Press Ctrl+C to stop the search at any time\x1b[0m\n");
    
    let _start = Instant::now();
    let addresses_checked = Arc::new(AtomicUsize::new(0));
    let found_count = Arc::new(AtomicUsize::new(0));
    
    // Create a vector to hold thread handles
    let mut handles = vec![];
    
    // Spawn worker threads
    for _ in 0..num_cpus::get() {
        let pattern = pattern.clone();
        let position = position.clone();
        let results = results.clone();
        let progress = progress.clone();
        let found_count = found_count.clone();
        let addresses_checked = addresses_checked.clone();
        
        let handle = tokio::spawn(async move {
            loop {
                addresses_checked.fetch_add(1, Ordering::SeqCst);
                
                // Generate a random private key
                let private_key = generate_random_private_key();
                
                // Get the address from the private key
                let address_result = get_address_from_private_key_case_sensitive(&private_key, case_sensitive);
                
                // Update progress bar
                {
                    let progress_bar = progress.lock().unwrap();
                    progress_bar.set_position(addresses_checked.load(Ordering::SeqCst) as u64);
                    progress_bar.set_message(format!("{}", found_count.load(Ordering::SeqCst)));
                }
                
                if let Ok(address) = address_result {
                    // Check if the address matches the pattern
                    if matches_pattern(&address, &pattern, &position, case_sensitive) {
                        // Increment the found count
                        found_count.fetch_add(1, Ordering::SeqCst);
                        
                        // Add the address and private key to the results
                        results.lock().unwrap().push((
                            if case_sensitive { convert_to_mixed_case(&address) } else { address.clone() },
                            format!("0x{}", private_key)
                        ));
                        
                        // Let's find at most 5 addresses
                        if found_count.load(Ordering::SeqCst) >= 5 {
                            break;
                        }
                    }
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for any thread to complete (when enough addresses are found)
    for handle in handles {
        if results.lock().unwrap().len() >= 5 { // Fixed limit at 5
            break;
        }
        let _ = handle.await;
    }
    
    // Clear the progress bar before returning
    {
        let progress_bar = progress.lock().unwrap();
        progress_bar.finish_and_clear();
    }
    
    println!();  // Add a newline for spacing
    
    // Return a clone of the locked results before they go out of scope
    let result_clone = results.lock().unwrap().clone();
    result_clone
}

// Helper function to display results
fn display_results(results: &[(String, String)]) {
    if !results.is_empty() {
        println!("\n\x1b[1;32m✅ Found {} matching addresses!\x1b[0m", results.len());
        
        println!("\n\x1b[1;32m╔════════════════════════════════════════════════════╗");
        println!("║              MATCHING ADDRESSES                    ║");
        println!("╚════════════════════════════════════════════════════╝\x1b[0m");
        
        for (i, (address, private_key)) in results.iter().enumerate() {
            println!("\x1b[1;32m╔════════════════════════════════════════════════════╗\x1b[0m");
            println!("\x1b[1;32m║\x1b[0m \x1b[1;32m#{:<4}\x1b[0m                                          \x1b[1;32m║\x1b[0m", i + 1);
            println!("\x1b[1;32m╠════════════════════════════════════════════════════╣\x1b[0m");
            println!("\x1b[1;32m║\x1b[0m \x1b[1;33m📫 Address:\x1b[0m                                     \x1b[1;32m║\x1b[0m");
            
            // Split long addresses to fit in the box
            let wrapped_address = textwrap::fill(address, 48);
            for line in wrapped_address.lines() {
                println!("\x1b[1;32m║\x1b[0m \x1b[0;36m{:<48}\x1b[0m \x1b[1;32m║\x1b[0m", line);
            }
            
            println!("\x1b[1;32m╠════════════════════════════════════════════════════╣\x1b[0m");
            println!("\x1b[1;32m║\x1b[0m \x1b[1;33m🔑 Private Key:\x1b[0m                                 \x1b[1;32m║\x1b[0m");
            
            // Split long private keys to fit in the box
            let wrapped_key = textwrap::fill(private_key, 48);
            for line in wrapped_key.lines() {
                println!("\x1b[1;32m║\x1b[0m \x1b[0;35m{:<48}\x1b[0m \x1b[1;32m║\x1b[0m", line);
            }
            
            println!("\x1b[1;32m╚════════════════════════════════════════════════════╝\x1b[0m");
        }
    } else {
        println!("\n\x1b[1;31m❌ No matching addresses found within the search limit.\x1b[0m");
    }
}

// Function to find addresses based on pattern
async fn find_addresses(pattern: String, position: Position, _threads: usize, case_sensitive: bool) -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Convert Position enum to String for the new function
    let position_str = match position {
        Position::Prefix => "prefix".to_string(),
        Position::Suffix => "suffix".to_string(),
        Position::Anywhere => "anywhere".to_string(),
    };
    
    // Call the async function and wait for it to complete
    let results = search_vanity_address(pattern, position_str, case_sensitive).await;
    
    // Display the results
    display_results(&results);
    
    Ok(())
}

// Function to validate a pattern for hex characters
fn is_valid_hex_pattern(pattern: &str) -> bool {
    pattern.chars().all(|c| c.is_digit(16))
}

// Function to check and warn about non-hex characters
fn warn_if_invalid_hex(pattern: &str) -> bool {
    if !is_valid_hex_pattern(pattern) {
        eprintln!("\n WARNING: Your pattern contains non-hexadecimal characters!");
        eprintln!("   Fuel addresses can only contain characters: 0-9, a-f");
        eprintln!("   The search may run indefinitely without finding a match.\n");
        
        // List the invalid characters
        let invalid_chars: Vec<char> = pattern.chars().filter(|c| !c.is_digit(16)).collect();
        eprintln!("   Invalid characters in your pattern: {:?}", invalid_chars);
        eprintln!("   Consider using only hexadecimal characters for a successful search.\n");
        
        return false;
    }
    true
}

// Function to display banner
fn display_banner() {
    println!("\n\
\x1b[1;32m██╗███████╗██╗   ██╗███████╗██╗     \x1b[0m\n\
\x1b[1;32m██║██╔════╝██║   ██║██╔════╝██║     \x1b[0m\n\
\x1b[1;32m██║█████╗  ██║   ██║█████╗  ██║     \x1b[0m\n\
\x1b[1;32m██║██╔══╝  ██║   ██║██╔══╝  ██║     \x1b[0m\n\
\x1b[1;32m██║██║     ╚██████╔╝███████╗███████╗\x1b[0m\n\
\x1b[1;32m╚═╝╚═╝      ╚═════╝ ╚══════╝╚══════╝\x1b[0m\n\
    ");
    println!("\x1b[1;32m════════════════════════════════════════════════════\x1b[0m");
    println!("\x1b[1;32m⚡ Fuel Vanity Address Generator v1.0.0 ⚡\x1b[0m");
    println!("\x1b[1;32m════════════════════════════════════════════════════\x1b[0m");
    println!("  Created by Ban (https://x.com/ohbannedOS)");
    println!("  https://github.com/ohbanned/Fuel-Vanity-Address-Generator");
    println!("\x1b[1;32m════════════════════════════════════════════════════\x1b[0m\n");
}

// Function to display help
fn display_help() {
    println!("\n\x1b[1;32m╔════════════════════════════════════════════╗");
    println!("║           iFuel COMMAND REFERENCE          ║");
    println!("╚════════════════════════════════════════════╝\x1b[0m");
    println!("\x1b[1;32m┌────────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m 🔍 GENERATION COMMANDS:                    \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m                                            \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m  prefix <pattern>                          \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m    Generate addresses with specified prefix\x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m                                            \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m  suffix <pattern>                          \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m    Generate addresses with specified suffix\x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m                                            \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m  contains <pattern>                        \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m    Generate addresses containing pattern   \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m└────────────────────────────────────────────┘\x1b[0m");
    println!("\x1b[1;32m┌────────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m ⚙️  OPTIONS:                               \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m                                            \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m  -s, --case-sensitive                      \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m    Enable case-sensitive pattern matching  \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m                                            \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m  -t, --threads <number>                    \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m    Set number of worker threads            \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m└────────────────────────────────────────────┘\x1b[0m");
    println!("\x1b[1;32m┌────────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m 📋 EXAMPLES:                               \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m                                            \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m  prefix abc                                \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m    Generate addresses starting with 'abc'  \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m                                            \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m  suffix cafe -s                            \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m    Generate case-sensitive address ending  \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m    with 'cafe'                             \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m└────────────────────────────────────────────┘\x1b[0m");
    println!("\x1b[1;32m┌────────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m 🛠️  UTILITY COMMANDS:                      \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m                                            \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m  info                                      \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m    Display this help message               \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m                                            \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m  exit                                      \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m│\x1b[0m    Exit the program                        \x1b[1;32m│\x1b[0m");
    println!("\x1b[1;32m└────────────────────────────────────────────┘\x1b[0m");
}

// Interactive mode
async fn interactive_mode(_threads: usize, case_sensitive: bool) -> std::result::Result<(), Box<dyn std::error::Error>> {
    display_banner();
    println!("💡 Type 'help' for available commands or 'exit' to quit.");
    println!("");
    
    loop {
        print!("\x1b[1;32miFuel>\x1b[0m ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let command = parse_input(&input, case_sensitive);
        
        match command {
            Some(Command::Exit) => {
                println!("\n👋 Thank you for using iFuel Vanity Address Generator!");
                println!("   Visit us at https://github.com/ohbanned/Fuel-Vanity-Address-Generator");
                break;
            },
            Some(cmd) => {
                // Avoid recursion issue by manually handling each command type
                match cmd {
                    Command::Prefix { pattern } => {
                        display_banner();
                        println!("⚙️  CONFIGURATION:");
                        println!("\x1b[1;32m┌────────────────────────────────────────────┐\x1b[0m");
                        println!("\x1b[1;32m│\x1b[0m Pattern type: \x1b[1;32mPrefix\x1b[0m                    \x1b[1;32m│\x1b[0m");
                        println!("\x1b[1;32m│\x1b[0m Pattern: \x1b[1;33m{:<32}\x1b[0m \x1b[1;32m│\x1b[0m", pattern);
                        println!("\x1b[1;32m│\x1b[0m Case-sensitive: \x1b[1;35m{:<23}\x1b[0m \x1b[1;32m│\x1b[0m", case_sensitive);
                        println!("\x1b[1;32m└────────────────────────────────────────────┘\x1b[0m");
                        println!("🔍 Searching for vanity addresses...");
                        println!("   Press Ctrl+C to stop at any time...\n");
                        
                        let position = "prefix".to_string();
                        let results = search_vanity_address(pattern, position, case_sensitive).await;
                        display_results(&results);
                    },
                    Command::Suffix { pattern } => {
                        display_banner();
                        println!("⚙️  CONFIGURATION:");
                        println!("\x1b[1;32m┌────────────────────────────────────────────┐\x1b[0m");
                        println!("\x1b[1;32m│\x1b[0m Pattern type: \x1b[1;32mSuffix\x1b[0m                    \x1b[1;32m│\x1b[0m");
                        println!("\x1b[1;32m│\x1b[0m Pattern: \x1b[1;33m{:<32}\x1b[0m \x1b[1;32m│\x1b[0m", pattern);
                        println!("\x1b[1;32m│\x1b[0m Case-sensitive: \x1b[1;35m{:<23}\x1b[0m \x1b[1;32m│\x1b[0m", case_sensitive);
                        println!("\x1b[1;32m└────────────────────────────────────────────┘\x1b[0m");
                        println!("🔍 Searching for vanity addresses...");
                        println!("   Press Ctrl+C to stop at any time...\n");
                        
                        let position = "suffix".to_string();
                        let results = search_vanity_address(pattern, position, case_sensitive).await;
                        display_results(&results);
                    },
                    Command::Contains { pattern } => {
                        display_banner();
                        println!("⚙️  CONFIGURATION:");
                        println!("\x1b[1;32m┌────────────────────────────────────────────┐\x1b[0m");
                        println!("\x1b[1;32m│\x1b[0m Pattern type: \x1b[1;32mContains\x1b[0m                  \x1b[1;32m│\x1b[0m");
                        println!("\x1b[1;32m│\x1b[0m Pattern: \x1b[1;33m{:<32}\x1b[0m \x1b[1;32m│\x1b[0m", pattern);
                        println!("\x1b[1;32m│\x1b[0m Case-sensitive: \x1b[1;35m{:<23}\x1b[0m \x1b[1;32m│\x1b[0m", case_sensitive);
                        println!("\x1b[1;32m└────────────────────────────────────────────┘\x1b[0m");
                        println!("🔍 Searching for vanity addresses...");
                        println!("   Press Ctrl+C to stop at any time...\n");
                        
                        let position = "anywhere".to_string();
                        let results = search_vanity_address(pattern, position, case_sensitive).await;
                        display_results(&results);
                    },
                    Command::Info => display_help(),
                    Command::Interactive => println!("\x1b[1;33mℹ️  You're already in interactive mode\x1b[0m"),
                    Command::Exit => break,
                }
            },
            None => continue,
        }
        
        println!("");
    }
    
    Ok(())
}

// Function to parse user input
fn parse_input(input: &str, _case_sensitive: bool) -> Option<Command> {
    let parts: Vec<&str> = input.trim().split_whitespace().collect();
    
    if parts.is_empty() {
        return None;
    }
    
    match parts[0].to_lowercase().as_str() {
        "prefix" => {
            if parts.len() < 2 {
                println!("Error: 'prefix' command requires a pattern");
                return None;
            }
            
            let pattern = parts[1].to_string();
            Some(Command::Prefix { pattern })
        },
        "suffix" => {
            if parts.len() < 2 {
                println!("Error: 'suffix' command requires a pattern");
                return None;
            }
            
            let pattern = parts[1].to_string();
            Some(Command::Suffix { pattern })
        },
        "contains" => {
            if parts.len() < 2 {
                println!("Error: 'contains' command requires a pattern");
                return None;
            }
            
            let pattern = parts[1].to_string();
            Some(Command::Contains { pattern })
        },
        "help" | "info" => Some(Command::Info),
        "exit" | "quit" => Some(Command::Exit),
        "interactive" => Some(Command::Interactive),
        _ => {
            println!("Unknown command: {}", parts[0]);
            println!("Type 'help' for a list of available commands");
            None
        }
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    if args.command.is_some() {
        // Execute a single command (non-interactive mode)
        if let Some(cmd) = args.command {
            display_banner();
            println!("Running command in non-interactive mode");
            execute_command(cmd, args.threads, args.case_sensitive).await?;
        }
    } else {
        // Interactive mode
        interactive_mode(args.threads, args.case_sensitive).await?;
    }
    
    Ok(())
}

// Function to execute a command
async fn execute_command(cmd: Command, threads: usize, case_sensitive: bool) -> std::result::Result<(), Box<dyn std::error::Error>> {
    match cmd {
        Command::Prefix { pattern } => {
            let position = "prefix".to_string();
            let results = search_vanity_address(pattern, position, case_sensitive).await;
            display_results(&results);
        },
        Command::Suffix { pattern } => {
            let position = "suffix".to_string();
            let results = search_vanity_address(pattern, position, case_sensitive).await;
            display_results(&results);
        },
        Command::Contains { pattern } => {
            let position = "anywhere".to_string();
            let results = search_vanity_address(pattern, position, case_sensitive).await;
            display_results(&results);
        },
        Command::Info => display_help(),
        Command::Interactive => interactive_mode(threads, case_sensitive).await?,
        Command::Exit => {}
    }
    
    Ok(())
}
