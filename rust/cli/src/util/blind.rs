use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::edwards::EdwardsPoint;
use curve25519_dalek::constants::ED25519_BASEPOINT_POINT;
use rand_core::OsRng;
use sha2::{Sha512, Digest};

/// Blinds a message using a random blinding factor.
pub fn blind_message(message: &[u8]) -> (Scalar, Scalar) {
    let blinding_factor = Scalar::random(&mut OsRng);
    let message_scalar = hash_to_scalar(message);
    let blinded_message = message_scalar * blinding_factor;
    (blinded_message, blinding_factor)
}

/// Signs a blinded message using the signing key.
pub fn sign_blinded_message(signing_key: &Scalar, blinded_message: &Scalar) -> (EdwardsPoint, Scalar) {
    let r = Scalar::random(&mut OsRng);
    let r_point = ED25519_BASEPOINT_POINT * r;
    let k = hash_to_scalar(r_point.compress().as_bytes());
    let s = r + k * signing_key * blinded_message;
    (r_point, s)
}

/// Unblinds a signature using the blinding factor.
pub fn unblind_signature(r: EdwardsPoint, s: Scalar, blinding_factor: &Scalar) -> (EdwardsPoint, Scalar) {
    let s_unblinded = s * blinding_factor.invert();
    (r, s_unblinded)
}

/// Verifies a signature against a public key and message.
pub fn verify_signature(
    public_key: &EdwardsPoint,
    message: &[u8],
    r: EdwardsPoint,
    s: Scalar,
) -> bool {
    let message_scalar = hash_to_scalar(message);
    let k = hash_to_scalar(r.compress().as_bytes());
    let left = ED25519_BASEPOINT_POINT * s;
    let right = r + (public_key * k);
    left == right * message_scalar.invert()
}

/// Helper function to hash a byte slice to a Scalar.
fn hash_to_scalar(data: &[u8]) -> Scalar {
    let mut hasher = Sha512::new();
    hasher.update(data);
    Scalar::from_hash(hasher)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blind_sign_unblind_verify() {
        let message = b"Hello, world!";
        
        // Generate a key pair
        let secret_key = Scalar::random(&mut OsRng);
        let public_key = ED25519_BASEPOINT_POINT * secret_key;

        // Blind the message
        let (blinded_message, blinding_factor) = blind_message(message);

        // Sign the blinded message
        let (r, s_blinded) = sign_blinded_message(&secret_key, &blinded_message);

        // Unblind the signature
        let (r, s) = unblind_signature(r, s_blinded, &blinding_factor);

        // Verify the signature
        let verification_result = verify_signature(&public_key, message, r, s);
        println!("Verification result: {}", verification_result);
        assert!(verification_result, "Signature verification failed");

        // Verify that the signature fails with a different message
        let wrong_message = b"Wrong message";
        assert!(!verify_signature(&public_key, wrong_message, r, s), "Signature incorrectly verified with wrong message");

        // Verify that the signature fails with a different public key
        let wrong_secret_key = Scalar::random(&mut OsRng);
        let wrong_public_key = ED25519_BASEPOINT_POINT * wrong_secret_key;
        assert!(!verify_signature(&wrong_public_key, message, r, s), "Signature incorrectly verified with wrong public key");
    }

    #[test]
    fn test_different_blinding_factors_produce_different_results() {
        let message = b"Hello, world!";
        
        let (blinded_message1, _) = blind_message(message);
        let (blinded_message2, _) = blind_message(message);

        assert_ne!(blinded_message1, blinded_message2, "Blinded messages should be different");
    }

    #[test]
    fn test_unblind_signature_correctness() {
        let message = b"Hello, world!";
        let secret_key = Scalar::random(&mut OsRng);
        let public_key = ED25519_BASEPOINT_POINT * secret_key;

        let (blinded_message, blinding_factor) = blind_message(message);
        let (r, s_blinded) = sign_blinded_message(&secret_key, &blinded_message);
        let (r_unblinded, s_unblinded) = unblind_signature(r, s_blinded, &blinding_factor);

        assert_eq!(r, r_unblinded, "R point should not change during unblinding");
        assert_ne!(s_blinded, s_unblinded, "S scalar should change during unblinding");
        
        let verification_result = verify_signature(&public_key, message, r_unblinded, s_unblinded);
        println!("Unblinded signature verification result: {}", verification_result);
        assert!(verification_result, "Unblinded signature should verify correctly");
    }

    #[test]
    fn test_blind_signature_process() {
        let message = b"Hello, world!";
        
        // Generate a key pair
        let secret_key = Scalar::random(&mut OsRng);
        let public_key = ED25519_BASEPOINT_POINT * secret_key;

        // Blind the message
        let (blinded_message, blinding_factor) = blind_message(message);

        // Sign the blinded message
        let (r, s_blinded) = sign_blinded_message(&secret_key, &blinded_message);

        // Unblind the signature
        let (r, s) = unblind_signature(r, s_blinded, &blinding_factor);

        // Verify the signature
        let verification_result = verify_signature(&public_key, message, r, s);
        
        println!("Original message: {:?}", message);
        println!("Blinded message: {:?}", blinded_message);
        println!("Blinding factor: {:?}", blinding_factor);
        println!("R: {:?}", r);
        println!("S (blinded): {:?}", s_blinded);
        println!("S (unblinded): {:?}", s);
        println!("Verification result: {}", verification_result);

        assert!(verification_result, "Blind signature process failed");
    }
}
