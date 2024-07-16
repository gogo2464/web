use super::*;

use log::{debug, error};

pub fn sign_with_key(blinded_verifying_key: &Value, server_master_signing_key: &str) -> Result<String, CryptoError> {
    debug!("Entering sign_with_key function");
    debug!("Server master signing key: {}", server_master_signing_key);
    debug!("Blinded verifying key: {:?}", blinded_verifying_key);

    let decoded_key = extract_bytes_from_armor(server_master_signing_key, "MASTER SIGNING KEY")?;
    debug!("Extracted bytes from armor: {:?}", decoded_key);

    let decoded_key = general_purpose::STANDARD.decode(&decoded_key)
        .map_err(|e| {
            error!("Failed to decode extracted bytes: {}", e);
            CryptoError::Base64DecodeError(e.to_string())
        })?;
    debug!("Decoded extracted bytes: {:?}", decoded_key);

    let master_signing_key = SigningKey::from_slice(&decoded_key)
        .map_err(|e| {
            error!("Failed to create SigningKey: {}", e);
            CryptoError::KeyCreationError(e.to_string())
        })?;
    debug!("Created master signing key");

    let blinded_verifying_key_bytes = match blinded_verifying_key {
        Value::String(s) => {
            debug!("Blinded verifying key is a string: {}", s);
            general_purpose::STANDARD.decode(s)
                .map_err(|e| {
                    error!("Failed to decode blinded verifying key: {}", e);
                    CryptoError::Base64DecodeError(e.to_string())
                })?
        },
        Value::Object(obj) => {
            let x = obj.get("x").and_then(Value::as_str)
                .ok_or_else(|| CryptoError::InvalidInput("Missing 'x' coordinate".to_string()))?;
            let y = obj.get("y").and_then(Value::as_str)
                .ok_or_else(|| CryptoError::InvalidInput("Missing 'y' coordinate".to_string()))?;

            let mut bytes = Vec::new();
            bytes.extend_from_slice(&general_purpose::STANDARD.decode(x)
                .map_err(|e| CryptoError::Base64DecodeError(format!("Failed to decode 'x' coordinate: {}", e)))?);
            bytes.extend_from_slice(&general_purpose::STANDARD.decode(y)
                .map_err(|e| CryptoError::Base64DecodeError(format!("Failed to decode 'y' coordinate: {}", e)))?);
            bytes
        },
        _ => return Err(CryptoError::InvalidInput("Invalid blinded verifying key format".to_string())),
    };

    // Generate a random nonce
    let nonce = SecretKey::random(&mut OsRng);
    let nonce_bytes = nonce.to_bytes();

    // Combine the blinded verifying key and nonce, and hash them
    let mut hasher = Sha256::new();
    hasher.update(&blinded_verifying_key_bytes);
    hasher.update(&nonce_bytes);
    let message = hasher.finalize();

    // Sign the hash
    let blind_signature: ecdsa::Signature = master_signing_key.sign(&message);

    // Combine the signature and nonce
    let mut combined = blind_signature.to_vec();
    combined.extend_from_slice(&nonce_bytes);

    Ok(general_purpose::STANDARD.encode(combined))
}
