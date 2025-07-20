use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("Invalid key length")]
    InvalidKeyLength,
}

/// A service to handle symmetric encryption for notification channel configurations.
/// Uses AES-256-GCM.
pub struct EncryptionService {
    // The cipher is created from a 32-byte key.
    cipher: Aes256Gcm,
}

impl EncryptionService {
    /// Creates a new EncryptionService with a 32-byte key.
    /// The key should be loaded securely, e.g., from an environment variable.
    pub fn new(key: &[u8]) -> Result<Self, EncryptionError> {
        Ok(Self {
            cipher: Aes256Gcm::new_from_slice(key)
                .map_err(|_e| EncryptionError::InvalidKeyLength)?, // Renamed e to _e
        })
    }

    /// Encrypts a plaintext byte slice.
    /// Prepends a 12-byte (96-bit) nonce to the ciphertext.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; must be unique for each encryption
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

        // Prepend nonce to the ciphertext. The nonce is required for decryption.
        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    /// Decrypts an encrypted byte slice.
    /// Assumes the first 12 bytes are the nonce.
    pub fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        if encrypted_data.len() < 12 {
            return Err(EncryptionError::DecryptionFailed(
                "Invalid encrypted data: too short to contain a nonce".to_string(),
            ));
        }

        let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;

        Ok(plaintext)
    }
}
