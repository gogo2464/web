use p256::{
    ecdsa::{SigningKey, Signature, signature::Verifier, VerifyingKey, signature::Signer},
    PublicKey,
};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DelegatedKeyMetadata {
    pub creation_date: DateTime<Utc>,
    pub purpose: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DelegatedKey {
    pub public_key: Vec<u8>,
    pub metadata: DelegatedKeyMetadata,
    pub master_signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Certificate {
    pub delegated_key: DelegatedKey,
    pub certified_public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

pub fn sign_certificate(delegated_key: &DelegatedKey, public_key: &PublicKey) -> Certificate {
    let signing_key = SigningKey::from_slice(&delegated_key.public_key).unwrap();
    
    let signature = <SigningKey as Signer<Signature>>::sign(&signing_key, public_key.to_sec1_bytes().as_ref()).to_vec();

    Certificate {
        delegated_key: delegated_key.clone(),
        certified_public_key: public_key.to_sec1_bytes().to_vec(),
        signature,
    }
}

pub fn verify_certificate(cert: &Certificate, master_public_key: &VerifyingKey) -> bool {
    // Verify master signature on delegated key
    let mut buf = Vec::new();
    buf.extend_from_slice(&serde_json::to_vec(&cert.delegated_key.metadata).unwrap());
    buf.extend_from_slice(&cert.delegated_key.public_key);
    
    if master_public_key.verify(&buf, &Signature::from_slice(&cert.delegated_key.master_signature).unwrap()).is_err() {
        return false;
    }

    // Verify delegated key signature on certified public key
    let delegated_verifying_key = VerifyingKey::from_sec1_bytes(&cert.delegated_key.public_key).unwrap();
    delegated_verifying_key.verify(&cert.certified_public_key, &Signature::from_slice(&cert.signature).unwrap()).is_ok()
}

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    GenerateKey {
        #[arg(short, long)]
        purpose: String,
    },
    SignCertificate {
        #[arg(short, long)]
        public_key: String,
    },
    VerifyCertificate {
        #[arg(short, long)]
        certificate: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::GenerateKey { purpose }) => {
            println!("Generating key with purpose: {}", purpose);
            // Add key generation logic here
        }
        Some(Commands::SignCertificate { public_key }) => {
            println!("Signing certificate for public key: {}", public_key);
            // Add certificate signing logic here
        }
        Some(Commands::VerifyCertificate { certificate }) => {
            println!("Verifying certificate: {}", certificate);
            // Add certificate verification logic here
        }
        None => {
            println!("No command specified. Use --help for usage information.");
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_key_generation() {
        // Add your test logic here
        println!("Running test code...");
    }
}
