use p256::ecdsa::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use p256::ecdsa::{self, signature::{Signer, Verifier}};
use crate::armor;
use serde::{Serialize, Deserialize};
use rmp_serde::Serializer;
use crate::crypto::{CryptoError, extract_bytes_from_armor};
use rmp_serde;
use log::{debug, info, warn, error};
use colored::Colorize;

#[derive(Serialize, Deserialize, Debug)]
struct DelegateCertificate {
    delegate_verifying_key: String,
    // Add other fields as needed
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GhostKey {
    pub version: u8,
    pub certificate: GhostkeyCertificate,
    pub verifying_key: Vec<u8>,
    pub signing_key: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DelegateKeyCertificate {
    pub verifying_key: Vec<u8>,
    pub info: String,
    pub signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GhostkeyCertificate {
    pub version: u8,
    pub delegate_certificate: Vec<u8>,
    pub ghostkey_verifying_key: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GhostkeySigningData {
    pub version: u8,
    pub delegate_certificate: Vec<u8>,
    pub ghostkey_verifying_key: Vec<u8>,
}

pub fn generate_ghostkey(delegate_certificate: &str, delegate_signing_key: &str) -> Result<String, CryptoError> {
    info!("Generating ghostkey");
    
    // Extract the delegate certificate bytes
    let delegate_certificate_bytes = extract_bytes_from_armor(delegate_certificate, "DELEGATE CERTIFICATE")?;
    debug!("Delegate certificate bytes: {:?}", delegate_certificate_bytes);

    // Extract the delegate signing key
    let delegate_signing_key_bytes = extract_bytes_from_armor(delegate_signing_key, "DELEGATE SIGNING KEY")?;
    let delegate_signing_key = SigningKey::from_slice(&delegate_signing_key_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;
    debug!("Extracted delegate signing key");

    // Generate the ghostkey key pair
    let ghostkey_signing_key = SigningKey::random(&mut OsRng);
    let ghostkey_verifying_key = VerifyingKey::from(&ghostkey_signing_key);
    debug!("Generated ghostkey verifying key: {:?}", ghostkey_verifying_key.to_encoded_point(false));

    // Create the signing data
    let ghostkey_signing_data = GhostkeySigningData {
        version: 1,
        delegate_certificate: delegate_certificate_bytes.clone(),
        ghostkey_verifying_key: ghostkey_verifying_key.to_sec1_bytes().to_vec(),
    };

    // Serialize the signing data to MessagePack
    let mut buf = Vec::new();
    ghostkey_signing_data.serialize(&mut Serializer::new(&mut buf))
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;
    debug!("Serialized signing data: {:?}", buf);

    // Sign the serialized data with the delegate signing key
    let signature: ecdsa::Signature = delegate_signing_key.sign(&buf);
    debug!("Generated signature: {:?}", signature);

    // Create the final certificate with the signature
    let final_certificate = GhostkeyCertificate {
        version: 1,
        delegate_certificate: delegate_certificate_bytes,
        ghostkey_verifying_key: ghostkey_signing_data.ghostkey_verifying_key,
        signature: signature.to_der().as_bytes().to_vec(),
    };

    // Serialize the final certificate to MessagePack
    let mut final_buf = Vec::new();
    final_certificate.serialize(&mut Serializer::new(&mut final_buf))
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;
    debug!("Serialized final certificate: {:?}", final_buf);

    // Armor the certificate
    let ghostkey_certificate_armored = armor(&final_buf, "GHOSTKEY CERTIFICATE", "GHOSTKEY CERTIFICATE");

    // Armor the ghost key
    let ghost_key_armored = armor(&ghostkey_signing_key.to_bytes(), "GHOST KEY", "GHOST KEY");

    // Combine the armored certificate and ghost key
    let formatted_output = format!("{}\n\n{}", ghostkey_certificate_armored, ghost_key_armored);

    debug!("Formatted Ghost Key: {}", formatted_output);

    Ok(formatted_output)
}


pub fn validate_delegate_key(master_verifying_key_pem: &str, delegate_certificate_armored: &str, delegate_certificate_file: &str) -> Result<String, CryptoError> {
    info!("Starting validate_delegate_key function");
    
    // Extract the base64 encoded delegate certificate
    let delegate_certificate_bytes = extract_delegate_certificate(delegate_certificate_armored)?;

    debug!("Extracted delegate certificate bytes: {:?}", delegate_certificate_bytes);
    info!("Extracted delegate certificate length: {}", delegate_certificate_bytes.len());

    // Validate the delegate certificate using the master verifying key
    info!("Validating delegate certificate");
    let delegate_info = validate_delegate_certificate(master_verifying_key_pem, &delegate_certificate_bytes)?;
    info!("Delegate certificate validated successfully");

    info!("{}", "Delegate key certificate is valid.".green().bold());

    Ok(delegate_info)
}

pub fn validate_delegate_certificate(master_verifying_key_pem: &str, delegate_certificate: &[u8]) -> Result<String, CryptoError> {
    info!("Validating delegate certificate");
    
    // Extract the base64 encoded master verifying key
    let master_verifying_key_bytes = extract_bytes_from_armor(master_verifying_key_pem, "MASTER VERIFYING KEY")?;
    debug!("Master verifying key bytes: {:?}", master_verifying_key_bytes);
    
    let master_verifying_key = VerifyingKey::from_sec1_bytes(&master_verifying_key_bytes)
        .map_err(|e| {
            error!("Failed to create VerifyingKey: {:?}", e);
            CryptoError::KeyCreationError(e.to_string())
        })?;

    // Deserialize the delegate certificate
    let delegate_cert: DelegateKeyCertificate = rmp_serde::from_slice(delegate_certificate)
        .map_err(|e| {
            error!("Deserialization error: {:?}", e);
            debug!("Delegate certificate bytes: {:?}", delegate_certificate);
            CryptoError::DeserializationError(e.to_string())
        })?;

    debug!("Deserialized delegate certificate: {:?}", delegate_cert);

    // Recreate the certificate data that was originally signed
    let certificate_data = DelegateKeyCertificate {
        verifying_key: delegate_cert.verifying_key.clone(),
        info: delegate_cert.info.clone(),
        signature: vec![],
    };

    // Serialize the certificate data
    let buf = rmp_serde::to_vec(&certificate_data)
        .map_err(|e| {
            error!("Failed to serialize certificate data: {:?}", e);
            CryptoError::SerializationError(e.to_string())
        })?;

    debug!("Serialized certificate data: {:?}", buf);

    // Verify the signature
    let signature = match ecdsa::Signature::from_der(&delegate_cert.signature) {
        Ok(sig) => {
            debug!("Successfully created Signature from DER");
            sig
        },
        Err(e) => {
            warn!("Failed to create Signature from DER: {:?}", e);
            debug!("DER-encoded signature: {:?}", delegate_cert.signature);
            // Try to create signature from raw bytes as a fallback
            match ecdsa::Signature::try_from(delegate_cert.signature.as_slice()) {
                Ok(sig) => {
                    debug!("Successfully created Signature from raw bytes");
                    sig
                },
                Err(e) => {
                    error!("Failed to create Signature from raw bytes: {:?}", e);
                    return Err(CryptoError::SignatureError(format!("Failed to create Signature from DER and raw bytes: {:?}", e)));
                }
            }
        }
    };

    debug!("Signature: {:?}", signature);

    match master_verifying_key.verify(&buf, &signature) {
        Ok(_) => {
            info!("Signature verified successfully");
            Ok(delegate_cert.info)
        },
        Err(e) => {
            error!("Signature verification failed: {:?}", e);
            debug!("Data being verified: {:?}", buf);
            debug!("Signature being verified: {:?}", signature);
            debug!("Master verifying key: {:?}", master_verifying_key.to_encoded_point(false));
            Err(CryptoError::SignatureVerificationError(format!("Signature verification failed: {:?}", e)))
        }
    }
}

pub fn verify_ghostkey_signature(ghostkey_certificate: &GhostkeyCertificate) -> Result<(), CryptoError> {
    info!("Verifying ghostkey signature");
    
    // Extract the delegate certificate bytes
    let delegate_certificate_bytes = if ghostkey_certificate.delegate_certificate.starts_with(b"-----BEGIN DELEGATE CERTIFICATE-----") {
        extract_bytes_from_armor(&String::from_utf8_lossy(&ghostkey_certificate.delegate_certificate), "DELEGATE CERTIFICATE")?
    } else {
        ghostkey_certificate.delegate_certificate.clone()
    };
    
    // Extract the delegate verifying key from the delegate certificate
    let delegate_verifying_key = extract_delegate_verifying_key(&delegate_certificate_bytes)?;
    debug!("Extracted delegate verifying key: {:?}", delegate_verifying_key.to_encoded_point(false));

    // Recreate the certificate data that was originally signed
    let ghostkey_signing_data = GhostkeySigningData {
        version: 1,
        delegate_certificate: delegate_certificate_bytes,
        ghostkey_verifying_key: ghostkey_certificate.ghostkey_verifying_key.clone(),
    };
    debug!("Recreated ghostkey signing data: {:?}", ghostkey_signing_data);

    // Serialize the ghostkey signing data
    let buf = rmp_serde::to_vec(&ghostkey_signing_data)
        .map_err(|e| {
            error!("Failed to serialize ghostkey signing data: {:?}", e);
            CryptoError::SerializationError(e.to_string())
        })?;
    debug!("Serialized ghostkey signing data: {:?}", buf);

    // Create the signature from the stored bytes
    let signature = ecdsa::Signature::from_der(&ghostkey_certificate.signature)
        .or_else(|e| {
            warn!("Failed to create signature from DER: {:?}", e);
            if ghostkey_certificate.signature.len() != 64 {
                error!("Invalid signature length: {}", ghostkey_certificate.signature.len());
                return Err(CryptoError::SignatureError("Invalid signature length".to_string()));
            }
            let bytes: [u8; 64] = ghostkey_certificate.signature[..64].try_into()
                .map_err(|_| CryptoError::SignatureError("Failed to convert signature to array".to_string()))?;
            ecdsa::Signature::from_slice(&bytes)
                .map_err(|e| {
                    error!("Failed to create signature from bytes: {:?}", e);
                    CryptoError::SignatureError(format!("Failed to create signature from bytes: {}", e))
                })
        })
        .map_err(|e| {
            error!("Failed to create signature: {:?}", e);
            e
        })?;
    debug!("Created signature: {:?}", signature);

    // Verify the signature using the delegate verifying key
    match delegate_verifying_key.verify(&buf, &signature) {
        Ok(_) => {
            info!("Signature verified successfully");
            Ok(())
        },
        Err(e) => {
            error!("Signature verification failed: {:?}", e);
            debug!("Delegate verifying key: {:?}", delegate_verifying_key.to_encoded_point(false));
            debug!("Data being verified: {:?}", buf);
            debug!("Signature being verified: {:?}", signature);
            Err(CryptoError::SignatureVerificationError(e.to_string()))
        }
    }
}

pub fn extract_delegate_verifying_key(delegate_certificate: &[u8]) -> Result<VerifyingKey, CryptoError> {
    let delegate_cert: DelegateKeyCertificate = rmp_serde::from_slice(delegate_certificate)
        .map_err(|e| CryptoError::DeserializationError(e.to_string()))?;

    VerifyingKey::from_sec1_bytes(&delegate_cert.verifying_key)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))
}
/// Validates an armored ghost key certificate using the provided master verifying key.
///
/// # Arguments
///
/// * `master_verifying_key_pem` - The master verifying key in PEM format
/// * `ghostkey_certificate_armored` - The ghost key certificate in armored format
///
/// # Returns
///
/// The delegate info as a string if validation is successful, or a CryptoError if validation fails.
pub fn validate_armored_ghost_key_command(master_verifying_key_pem: &str, ghostkey_certificate_armored: &str, ghost_certificate_file: &str) -> Result<(), CryptoError> {
    info!("Starting validate_armored_ghost_key_command");
    match validate_ghost_key(master_verifying_key_pem, ghostkey_certificate_armored, ghost_certificate_file) {
        Ok(delegate_info) => {
            println!("{}", "Ghost key certificate validation successful.".green().bold());
            println!("{} {}", "Delegate info:".cyan(), delegate_info);
            Ok(())
        },
        Err(e) => {
            let error_message = match e {
                CryptoError::ArmorError(msg) => format!("The ghost key certificate could not be decoded: {}. Please check if it's properly formatted.", msg),
                CryptoError::DeserializationError(msg) => format!("The ghost key certificate format is invalid: {}. It may be corrupted or incompatible.", msg),
                CryptoError::KeyCreationError(msg) => format!("There's an issue with the master verifying key: {}. Please verify its correctness and try again.", msg),
                CryptoError::SignatureVerificationError(msg) => format!("The ghost key certificate signature is invalid: {}. This may indicate tampering or use of an incorrect master key.", msg),
                CryptoError::ValidationError(msg) => format!("Validation error: {}. Please check the certificate and master key.", msg),
                _ => format!("An unexpected error occurred during ghost key validation: {:?}. If this persists, please contact support.", e),
            };
            error!("Validation error: {}", error_message);
            eprintln!("{} {}", "Error:".red().bold(), error_message.red());
            Err(CryptoError::ValidationError(error_message))
        }
    }
}
fn extract_delegate_certificate(armored_text: &str) -> Result<Vec<u8>, CryptoError> {
    crate::crypto::extract_bytes_from_armor(armored_text, "DELEGATE CERTIFICATE")
}
