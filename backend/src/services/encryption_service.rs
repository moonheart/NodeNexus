use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use hex;

const NONCE_SIZE: usize = 12; // AES-GCM standard nonce size

pub fn encrypt(plain_text: &str, key_hex: &str) -> Result<String, String> {
    let key_bytes = hex::decode(key_hex).map_err(|e| format!("Invalid hex key: {e}"))?;
    if key_bytes.len() != 32 {
        return Err("Encryption key must be 32 bytes (256 bits) long".to_string());
    }
    let key = key_bytes.as_slice().into();
    let cipher = Aes256Gcm::new(key);

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plain_text.as_bytes())
        .map_err(|e| format!("Encryption failed: {e}"))?;

    let mut result = nonce.to_vec();
    result.extend_from_slice(&ciphertext);

    Ok(hex::encode(result))
}

pub fn decrypt(cipher_hex: &str, key_hex: &str) -> Result<String, String> {
    let key_bytes = hex::decode(key_hex).map_err(|e| format!("Invalid hex key: {e}"))?;
    if key_bytes.len() != 32 {
        return Err("Decryption key must be 32 bytes (256 bits) long".to_string());
    }
    let key = key_bytes.as_slice().into();
    let cipher = Aes256Gcm::new(key);

    let encrypted_data =
        hex::decode(cipher_hex).map_err(|e| format!("Invalid hex ciphertext: {e}"))?;
    if encrypted_data.len() < NONCE_SIZE {
        return Err("Ciphertext is too short to contain a nonce".to_string());
    }

    let (nonce_bytes, ciphertext) = encrypted_data.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    let decrypted_bytes = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {e}"))?;

    String::from_utf8(decrypted_bytes).map_err(|e| format!("Invalid UTF-8 sequence: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_success() {
        let key_hex = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"; // 32 bytes
        let plain_text = "This is a secret message.";

        let encrypted = encrypt(plain_text, key_hex).unwrap();
        let decrypted = decrypt(&encrypted, key_hex).unwrap();

        assert_ne!(plain_text, encrypted);
        assert_eq!(plain_text, decrypted);
    }

    #[test]
    fn test_decrypt_with_wrong_key() {
        let key1_hex = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        let key2_hex = "f1e1d1c1b1a191817161514131211101f0e0d0c0b0a090807060504030201000";
        let plain_text = "another secret";

        let encrypted = encrypt(plain_text, key1_hex).unwrap();
        let result = decrypt(&encrypted, key2_hex);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Decryption failed: aead::Error");
    }

    #[test]
    fn test_invalid_key_length() {
        let short_key = "1234";
        let long_key = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
        let plain_text = "test";

        let enc_result_short = encrypt(plain_text, short_key);
        assert!(enc_result_short.is_err());
        assert_eq!(
            enc_result_short.unwrap_err(),
            "Encryption key must be 32 bytes (256 bits) long"
        );

        let enc_result_long = encrypt(plain_text, long_key);
        assert!(enc_result_long.is_err());
        assert_eq!(
            enc_result_long.unwrap_err(),
            "Encryption key must be 32 bytes (256 bits) long"
        );

        let dec_result_short = decrypt("someciphertext", short_key);
        assert!(dec_result_short.is_err());
        assert_eq!(
            dec_result_short.unwrap_err(),
            "Decryption key must be 32 bytes (256 bits) long"
        );
    }

    #[test]
    fn test_invalid_hex() {
        let invalid_key = "not-a-hex-string";
        let valid_key = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        let plain_text = "test";

        let result = encrypt(plain_text, invalid_key);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid hex key"));

        let result_dec = decrypt("someciphertext", invalid_key);
        assert!(result_dec.is_err());
        assert!(result_dec.unwrap_err().contains("Invalid hex key"));

        let result_dec_invalid_cipher = decrypt("not-a-hex-cipher", valid_key);
        assert!(result_dec_invalid_cipher.is_err());
        assert!(
            result_dec_invalid_cipher
                .unwrap_err()
                .contains("Invalid hex ciphertext")
        );
    }
}
