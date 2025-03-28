use fuel_crypto::{PublicKey, SecretKey};
use rand::rngs::OsRng;
use rand::RngCore;
use sha3::{Digest, Keccak256};
use hex;

pub struct VanitySearchResult {
    pub private_key: String,
    pub address: String,
}

// Core functionality for generating and validating wallet addresses
pub fn search_vanity_address(
    pattern: &str, 
    position: &str, 
    case_sensitive: bool,
    max_addresses: u32
) -> Vec<VanitySearchResult> {
    let mut results = Vec::new();
    
    let lowercase_pattern = if !case_sensitive {
        pattern.to_lowercase()
    } else {
        pattern.to_string()
    };
    
    let mut found_count = 0;
    
    while found_count < max_addresses {
        // Generate random private key
        let mut key_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut key_bytes);
        
        if let Ok(secret_key) = SecretKey::try_from(&key_bytes[..]) {
            // Get the public key from the secret key
            let public_key = PublicKey::from(&secret_key);
            
            // Convert to Ethereum-style address
            let hash = keccak256_hash(&public_key.as_ref());
            let address = format!("0x{}", hex::encode(&hash[12..]));
            
            // Check if the address matches the pattern based on position
            let address_to_check = if !case_sensitive {
                address.to_lowercase()
            } else {
                address.clone()
            };
            
            let is_match = match position {
                "prefix" => address_to_check.starts_with(&lowercase_pattern),
                "suffix" => address_to_check.ends_with(&lowercase_pattern),
                "contains" => address_to_check.contains(&lowercase_pattern),
                _ => false,
            };
            
            if is_match {
                // Found a match
                results.push(VanitySearchResult {
                    private_key: format!("0x{}", hex::encode(secret_key.as_ref())),
                    address,
                });
                
                found_count += 1;
            }
        }
    }
    
    results
}

// Add a method to verify that a given private key produces the expected address
pub fn verify_key_address_pair(private_key: &str, expected_address: &str) -> bool {
    // Remove 0x prefix if present
    let clean_key = private_key.trim_start_matches("0x");
    
    // Parse the private key
    if let Ok(bytes) = hex::decode(clean_key) {
        if let Ok(secret_key) = SecretKey::try_from(&bytes[..]) {
            // Get the public key from the secret key
            let public_key = PublicKey::from(&secret_key);
            
            // Convert to Ethereum-style address
            let hash = keccak256_hash(&public_key.as_ref());
            let address = format!("0x{}", hex::encode(&hash[12..]));
            
            return address.eq_ignore_ascii_case(expected_address);
        }
    }
    
    false
}

// Helper function to calculate keccak256 hash (for Ethereum-style addresses)
fn keccak256_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result[..]);
    hash
}
