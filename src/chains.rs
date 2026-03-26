use anyhow::{Result, anyhow};
use bip39::{Language, Mnemonic};
use sha2::{Digest, Sha512};

// EVM: use k256 + secp256k1 directly
use k256::ecdsa::SigningKey as EvmSigningKey;
use k256::Secp256k1;

// Ed25519 for Sui/Aptos
use ed25519_dalek::{SecretKey as EdSecretKey, SigningKey as EdSigningKey, VerifyingKey as EdVerifyingKey};

/// Derivation path types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DerivationPath {
    /// BIP44 standard path: m/44'/coin_type'/account'/change/address_index
    Standard,
    /// BIP49 P2WPKH-nested-in-P2SH: m/49'/coin_type'/account'/change/address_index
    SegWitP2SH,
    /// BIP84 P2WPKH native: m/84'/coin_type'/account'/change/address_index
    SegWitNative,
    /// Custom derivation path
    Custom(String),
}

impl DerivationPath {
    /// Get the purpose value for the derivation path
    pub fn purpose(&self) -> u32 {
        match self {
            DerivationPath::Standard => 44,
            DerivationPath::SegWitP2SH => 49,
            DerivationPath::SegWitNative => 84,
            DerivationPath::Custom(_) => 44, // Default to standard for custom
        }
    }
    
    /// Get the path as a string for display
    pub fn as_string(&self, coin_type: u32) -> String {
        match self {
            DerivationPath::Standard => format!("m/44'/{}'/0'/0/0", coin_type),
            DerivationPath::SegWitP2SH => format!("m/49'/{}'/0'/0/0", coin_type),
            DerivationPath::SegWitNative => format!("m/84'/{}'/0'/0/0", coin_type),
            DerivationPath::Custom(path) => path.clone(),
        }
    }
    
    /// Parse a custom path string
    pub fn from_string(path: &str) -> Result<Self> {
        if path.starts_with("m/44'/") {
            Ok(DerivationPath::Standard)
        } else if path.starts_with("m/49'/") {
            Ok(DerivationPath::SegWitP2SH)
        } else if path.starts_with("m/84'/") {
            Ok(DerivationPath::SegWitNative)
        } else {
            Ok(DerivationPath::Custom(path.to_string()))
        }
    }
}

/// Address derivation trait with derivation path support
pub trait DeriveAddress {
    fn derive_address(&self, seed: &bip39::Mnemonic, path: &DerivationPath) -> Result<String>;
}

/// Helper: HMAC-SHA512 for BIP32 derivation
fn hmac_sha512(key: &[u8], data: &[u8]) -> [u8; 64] {
    use hmac::Mac;
    let mut mac = hmac::Hmac::<sha2::Sha512>::new_from_slice(key)
        .expect("HMAC can take key of any size");
    mac.update(data);
    mac.finalize().into_bytes().try_into().expect("wrong length")
}

/// Helper: BIP32 derive child key
fn bip32_derive(parent_key: &[u8], index: u32, hardened: bool) -> Vec<u8> {
    let mut data = vec![0u8; 1 + 4];
    if hardened {
        data[0] = 0x00;
    }
    data[1..5].copy_from_slice(&index.to_be_bytes());
    if hardened {
        data.extend_from_slice(&parent_key[0..32]);
    } else {
        data.extend_from_slice(&parent_key[33..65]);
    }

    let i = hmac_sha512(&parent_key[32..64], &data);
    let il = &i[0..32];
    let ir = &i[32..64];
    let _ir = ir; // Unused for now

    let mut il_num = [0u8; 33];
    il_num[0] = 0;
    il_num[1..].copy_from_slice(il);

    let mut child_key = hmac_sha512(il, &il_num);
    child_key[0] ^= parent_key[0];

    child_key.to_vec()
}

/// EVM address derivation using k256
pub struct EvmDeriver;

impl DeriveAddress for EvmDeriver {
    fn derive_address(&self, seed: &bip39::Mnemonic, path: &DerivationPath) -> Result<String> {
        let phrase: String = seed.words().collect::<Vec<_>>().join(" ");

        // Generate seed from mnemonic
        let mut hasher = Sha512::new();
        hasher.update(phrase.as_bytes());
        let seed_bytes = hasher.finalize();

        // Parse derivation path
        let purpose = path.purpose();
        let coin_type = 60; // EVM coin type

        // BIP32 derivation: m/purpose'/coin_type'/account'/change/address_index
        let master = &seed_bytes[..64];
        let purpose_key = bip32_derive(master, purpose, true);
        let coin_key = bip32_derive(&purpose_key, coin_type, true);
        let account_key = bip32_derive(&coin_key, 0, true);
        let change_key = bip32_derive(&account_key, 0, false);
        let index_key = bip32_derive(&change_key, 0, false);

        // ECDSA key from private key
        let signing_key = EvmSigningKey::from_slice(&index_key[..32])
            .map_err(|e| anyhow!("Failed to create EVM signing key: {}", e))?;

        // Get verifying key and public key
        let verifying_key = signing_key.verifying_key();

        // Encode to uncompressed SEC1 format
        use k256::elliptic_curve::sec1::ToEncodedPoint;
        let encoded_point = verifying_key.to_encoded_point(false);
        let pubkey_bytes = encoded_point.as_bytes();

        // Skip the first byte (0x04 for uncompressed)
        let pubkey_no_prefix = &pubkey_bytes[1..];

        // Keccak-256 hash
        use sha3::{Digest as Sha3Digest, Keccak256};
        let mut hasher = Keccak256::new();
        hasher.update(pubkey_no_prefix);
        let hash = hasher.finalize();

        // Address: last 20 bytes of hash
        Ok(format!("0x{}", hex::encode(&hash[hash.len() - 20..])))
    }
}

/// Solana address derivation - Ed25519
pub struct SolanaDeriver;

impl DeriveAddress for SolanaDeriver {
    fn derive_address(&self, seed: &bip39::Mnemonic, path: &DerivationPath) -> Result<String> {
        let phrase: String = seed.words().collect::<Vec<_>>().join(" ");

        // Generate seed from mnemonic
        let mut hasher = Sha512::new();
        hasher.update(phrase.as_bytes());
        let seed_bytes = hasher.finalize();

        // Solana uses m/44'/501'/0'/0' (standard path)
        let purpose = 44; // Solana doesn't use different BIP paths
        let coin_type = 501;

        // Derive at m/44'/501'/0'/0' using simplified approach
        let master = &seed_bytes[..64];
        let purpose_key = bip32_derive(master, purpose, true);
        let coin_key = bip32_derive(&purpose_key, coin_type, true);
        let account_key = bip32_derive(&coin_key, 0, true);
        let change_key = bip32_derive(&account_key, 0, false);

        // Ed25519 keypair from derived private key
        let secret_key_bytes: [u8; 32] = change_key[..32].try_into()
            .map_err(|e| anyhow!("Failed to convert to [u8; 32]: {}", e))?;
        let signing_key = EdSigningKey::from_bytes(&secret_key_bytes);
        let verifying_key: EdVerifyingKey = signing_key.verifying_key();

        // Solana address: Base58 of Ed25519 public key (32 bytes)
        let pubkey_bytes = verifying_key.to_bytes();
        Ok(bs58::encode(&pubkey_bytes).into_string())
    }
}

/// Sui address derivation - Ed25519
pub struct SuiDeriver;

impl DeriveAddress for SuiDeriver {
    fn derive_address(&self, seed: &bip39::Mnemonic, path: &DerivationPath) -> Result<String> {
        let phrase: String = seed.words().collect::<Vec<_>>().join(" ");

        // Generate seed from mnemonic
        let mut hasher = Sha512::new();
        hasher.update(phrase.as_bytes());
        let seed_bytes = hasher.finalize();

        // Sui uses m/44'/784'/0'/0'/0' (standard path)
        let purpose = path.purpose(); // Allow different paths for testing
        let coin_type = 784;

        // Derive at m/purpose'/784'/0'/0'/0'
        let master = &seed_bytes[..64];
        let purpose_key = bip32_derive(master, purpose, true);
        let coin_key = bip32_derive(&purpose_key, coin_type, true);
        let account_key = bip32_derive(&coin_key, 0, true);
        let change_key = bip32_derive(&account_key, 0, false);
        let index_key = bip32_derive(&change_key, 0, false);

        // Ed25519 keypair from derived private key
        let secret_key_bytes: [u8; 32] = index_key[..32].try_into()
            .map_err(|e| anyhow!("Failed to convert to [u8; 32]: {}", e))?;
        let signing_key = EdSigningKey::from_bytes(&secret_key_bytes);
        let verifying_key: EdVerifyingKey = signing_key.verifying_key();

        // Sui address: hex-encoded Ed25519 public key (32 bytes) with 0x prefix
        let pubkey_bytes = verifying_key.to_bytes();
        Ok(format!("0x{}", hex::encode(pubkey_bytes)))
    }
}

/// Aptos address derivation - Ed25519
pub struct AptosDeriver;

impl DeriveAddress for AptosDeriver {
    fn derive_address(&self, seed: &bip39::Mnemonic, path: &DerivationPath) -> Result<String> {
        let phrase: String = seed.words().collect::<Vec<_>>().join(" ");

        // Generate seed from mnemonic
        let mut hasher = Sha512::new();
        hasher.update(phrase.as_bytes());
        let seed_bytes = hasher.finalize();

        // Aptos uses m/44'/637'/0'/0'/0' (standard path)
        let purpose = path.purpose(); // Allow different paths for testing
        let coin_type = 637;

        // Derive at m/purpose'/637'/0'/0'/0'
        let master = &seed_bytes[..64];
        let purpose_key = bip32_derive(master, purpose, true);
        let coin_key = bip32_derive(&purpose_key, coin_type, true);
        let account_key = bip32_derive(&coin_key, 0, true);
        let change_key = bip32_derive(&account_key, 0, false);
        let index_key = bip32_derive(&change_key, 0, false);

        // Ed25519 keypair from derived private key
        let secret_key_bytes: [u8; 32] = index_key[..32].try_into()
            .map_err(|e| anyhow!("Failed to convert to [u8; 32]: {}", e))?;
        let signing_key = EdSigningKey::from_bytes(&secret_key_bytes);
        let verifying_key: EdVerifyingKey = signing_key.verifying_key();

        // Aptos address: hex-encoded Ed25519 public key (32 bytes) with 0x prefix
        let pubkey_bytes = verifying_key.to_bytes();
        Ok(format!("0x{}", hex::encode(pubkey_bytes)))
    }
}

/// Blockchain network types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainType {
    EVM,
    Sui,
    Solana,
    Aptos,
    PiNetwork,
    Tron,
    Dogecoin,
}

/// Chain configuration with metadata
pub struct Chain {
    pub chain_type: ChainType,
    pub name: &'static str,
    pub icon: &'static str,
    pub derivation_path: &'static str,
    pub coin_type: u32,
    pub deriver: Box<dyn DeriveAddress + Send + Sync>,
}

impl Chain {
    pub fn from_type(chain_type: ChainType) -> Self {
        match chain_type {
            ChainType::EVM => Chain {
                chain_type,
                name: "EVM",
                icon: "🔷",
                derivation_path: "m/44'/60'/0'/0/0",
                coin_type: 60,
                deriver: Box::new(EvmDeriver),
            },
            ChainType::Sui => Chain {
                chain_type,
                name: "Sui",
                icon: "🌊",
                derivation_path: "m/44'/784'/0'/0'/0'",
                coin_type: 784,
                deriver: Box::new(SuiDeriver),
            },
            ChainType::Solana => Chain {
                chain_type,
                name: "Solana",
                icon: "☀️",
                derivation_path: "m/44'/501'/0'/0'",
                coin_type: 501,
                deriver: Box::new(SolanaDeriver),
            },
            ChainType::Aptos => Chain {
                chain_type,
                name: "Aptos",
                icon: "🅰️",
                derivation_path: "m/44'/637'/0'/0'/0'",
                coin_type: 637,
                deriver: Box::new(AptosDeriver),
            },
            ChainType::PiNetwork => Chain {
                chain_type,
                name: "Pi Network",
                icon: "π",
                derivation_path: "m/44'/911'/0'/0'/0'",
                coin_type: 911,
                deriver: Box::new(EvmDeriver), // Pi uses EVM-style derivation
            },
            ChainType::Tron => Chain {
                chain_type,
                name: "Tron",
                icon: "🔺",
                derivation_path: "m/44'/195'/0'/0/0",
                coin_type: 195,
                deriver: Box::new(EvmDeriver), // Tron uses ECDSA like EVM
            },
            ChainType::Dogecoin => Chain {
                chain_type,
                name: "Dogecoin",
                icon: "🐕",
                derivation_path: "m/44'/3'/0'/0/0",
                coin_type: 3,
                deriver: Box::new(EvmDeriver), // DOGE uses ECDSA
            },
        }
    }

    pub fn derive_address(&self, seed: &bip39::Mnemonic, path: &DerivationPath) -> Result<String> {
        self.deriver.derive_address(seed, path)
    }
    
    /// Get available derivation paths for this chain
    pub fn available_paths(&self) -> Vec<DerivationPath> {
        match self.chain_type {
            // EVM chains support all derivation paths
            ChainType::EVM | ChainType::PiNetwork | ChainType::Tron | ChainType::Dogecoin => {
                vec![
                    DerivationPath::Standard,
                    DerivationPath::SegWitP2SH,
                    DerivationPath::SegWitNative,
                ]
            },
            // Non-EVM chains typically use standard paths only
            ChainType::Sui | ChainType::Solana | ChainType::Aptos => {
                vec![DerivationPath::Standard]
            },
        }
    }
}