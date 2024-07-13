use p256::ecdsa::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use base64::{engine::general_purpose, Engine as _};
use p256::ecdsa::{self, signature::Signer};
use crate::armor;
use serde::{Serialize, Deserialize};
use rmp_serde::{Serializer};
use crate::crypto::{CryptoError, extract_base64_from_armor};

#[derive(Serialize, Deserialize)]
pub struct DelegateKeyCertificate {
    pub verifying_key: Vec<u8>,
    pub attributes: String,
    pub signature: Vec<u8>,
}

pub fn generate_delegate_key(master_signing_key_pem: &str, attributes: &str) -> Result<String, CryptoError> {
    let master_signing_key_base64 = extract_base64_from_armor(master_signing_key_pem, "MASTER SIGNING KEY")?;
    let master_signing_key_bytes = general_purpose::STANDARD.decode(&master_signing_key_base64)
        .map_err(|e| CryptoError::Base64DecodeError(e.to_string()))?;
    let master_signing_key = SigningKey::from_slice(&master_signing_key_bytes)
        .map_err(|e| CryptoError::KeyCreationError(e.to_string()))?;

    // Generate the delegate key pair
    let delegate_signing_key = SigningKey::random(&mut OsRng);
    let delegate_verifying_key = VerifyingKey::from(&delegate_signing_key);

    // Serialize the verifying key and attributes
    let verifying_key_bytes = delegate_verifying_key.to_encoded_point(false).as_bytes().to_vec();
    let certificate_data = DelegateKeyCertificate {
        verifying_key: verifying_key_bytes.clone(),
        attributes: attributes.to_string(),
        signature: vec![],
    };
    let mut buf = Vec::new();
    certificate_data.serialize(&mut Serializer::new(&mut buf))
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;
    let certificate_data_bytes = buf;

    // Sign the certificate data
    let signature: ecdsa::Signature = master_signing_key.sign(&certificate_data_bytes);
    let mut signed_certificate_data = certificate_data;
    signed_certificate_data.signature = signature.to_vec();

    // Serialize the signed certificate data to MessagePack
    let signed_certificate_msgpack = rmp_serde::to_vec(&signed_certificate_data)
        .map_err(|e| CryptoError::SerializationError(e.to_string()))?;

    // Encode the MessagePack data in base64
    let signed_certificate_base64 = general_purpose::STANDARD.encode(signed_certificate_msgpack);

    // Armor the signed certificate
    let armored_delegate_certificate = armor(signed_certificate_base64.as_bytes(), "DELEGATE CERTIFICATE", "DELEGATE CERTIFICATE");

    Ok(armored_delegate_certificate)
}
