use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use base64::Engine;
use hkdf::Hkdf;
use rand::{Rng, thread_rng};
use sha2::Sha256;
use std::env;

// helper to turn the Env String into a 32-byte Key
fn get_derived_key() -> [u8; 32] {
    let secret_string = env::var("APP_MASTER_KEY").expect("APP_MASTER_KEY must be set");

    let salt = None;

    // create the HKDF instance using SHA-256
    let hkdf = Hkdf::<Sha256>::new(salt, secret_string.as_bytes());

    // "info" is a context string. It prevents the same key from being used
    // in a different contexts if we decide to expand this system.
    let info = b"plethora-monitor-data-encryption";

    let mut result_key = [0u8; 32];

    hkdf.expand(info, &mut result_key)
        .expect("HKDF expansion failed");

    result_key
}

pub fn encrypt(plaintext: &str) -> String {
    let key = get_derived_key();
    let cipher = Aes256Gcm::new(&key.into());

    let mut nonce_bytes = [0u8; 12];
    thread_rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .expect("Encryption failure");

    let mut combined = nonce_bytes.to_vec();
    combined.extend(ciphertext);

    base64::engine::general_purpose::STANDARD.encode(combined)
}

pub fn decrypt(encrypted_val: &str) -> Result<String, anyhow::Error> {
    let key = get_derived_key();
    let cipher = Aes256Gcm::new(&key.into());

    let data = base64::engine::general_purpose::STANDARD.decode(encrypted_val)?;

    if data.len() < 12 {
        return Err(anyhow::anyhow!("Invalid encrypted data"));
    }
    let nonce_arr: [u8; 12] = data[0..12].try_into().expect("Invalid nonce length");
    let nonce = Nonce::from(nonce_arr);
    let ciphertext = &data[12..];

    let plaintext_bytes = cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("Decryption failed"))?;

    Ok(String::from_utf8(plaintext_bytes)?)
}

/// Generate a secure random verification token (32-byte hex string)
pub fn generate_verification_token() -> String {
    let bytes: [u8; 32] = thread_rng().r#gen();
    hex::encode(bytes)
}
