use clap::{Command, Arg};
use std::fs::{File, create_dir_all};
use std::io::Write;
use std::path::Path;
use common::crypto::{generate_master_key, generate_delegate_key, generate_signing_key, validate_delegate_key, sign_message};
use colored::Colorize;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("Freenet Key Utility")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Performs various Freenet-related tasks")
        .subcommand(Command::new("sign-message")
            .about("Signs a message using a signing key and outputs the signature")
            .arg(Arg::new("signing-key-file")
                .long("signing-key-file")
                .help("The file containing the signing key (master or delegate)")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("message")
                .long("message")
                .help("The message to sign")
                .required(true)
                .value_name("STRING"))
            .arg(Arg::new("output-file")
                .long("output-file")
                .help("The file to output the signature")
                .required(true)
                .value_name("FILE")))
        .subcommand(Command::new("generate-master-key")
            .about("Generates a new SERVER_MASTER_KEY and public key")
            .arg(Arg::new("output-dir")
                .long("output-dir")
                .help("The directory to output the keys")
                .required(true)
                .value_name("DIR")))
        .subcommand(Command::new("generate-delegate-key")
            .about("Generates a new delegate key and certificate")
            .arg(Arg::new("master-key-file")
                .long("master-key-file")
                .help("The file containing the master private key")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("attributes")
                .long("attributes")
                .help("The attributes string to be included in the delegate key certificate")
                .required(true)
                .value_name("STRING"))
            .arg(Arg::new("output-dir")
                .long("output-dir")
                .help("The directory to output the delegate keys and certificate")
                .required(true)
                .value_name("DIR")))
        .subcommand(Command::new("generate-signing-key")
            .about("Generates a new SERVER_SIGNING_KEY and public key")
            .arg(Arg::new("output-dir")
                .long("output-dir")
                .help("The directory to output the keys")
                .required(true)
                .value_name("DIR")))
        .subcommand(Command::new("validate-delegate-key")
            .about("Validates a delegate key certificate using the master verifying key")
            .arg(Arg::new("master-verifying-key-file")
                .long("master-verifying-key-file")
                .help("The file containing the master verifying key")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("delegate-certificate-file")
                .long("delegate-certificate-file")
                .help("The file containing the delegate certificate")
                .required(true)
                .value_name("FILE")))
        .get_matches();

    match matches.subcommand() {
        Some(("generate-master-key", sub_matches)) => {
            let output_dir = sub_matches.get_one::<String>("output-dir").unwrap();
            generate_and_save_master_key(output_dir)?;
        }
        Some(("generate-delegate-key", sub_matches)) => {
            let master_key_file = sub_matches.get_one::<String>("master-key-file").unwrap();
            let attributes = sub_matches.get_one::<String>("attributes").unwrap();
            let output_dir = sub_matches.get_one::<String>("output-dir").unwrap();
            generate_and_save_delegate_key(master_key_file, attributes, output_dir)?;
        }
        Some(("generate-signing-key", sub_matches)) => {
            let output_dir = sub_matches.get_one::<String>("output-dir").unwrap();
            generate_and_save_signing_key(output_dir)?;
        }
        Some(("validate-delegate-key", sub_matches)) => {
            let master_verifying_key_file = sub_matches.get_one::<String>("master-verifying-key-file").unwrap();
            let delegate_certificate_file = sub_matches.get_one::<String>("delegate-certificate-file").unwrap();
            validate_delegate_key_command(master_verifying_key_file, delegate_certificate_file)?;
        }
        Some(("sign-message", sub_matches)) => {
            let signing_key_file = sub_matches.get_one::<String>("signing-key-file").unwrap();
            let message = sub_matches.get_one::<String>("message").unwrap();
            let output_file = sub_matches.get_one::<String>("output-file").unwrap();
            sign_message_command(signing_key_file, message, output_file)?;
        }
        _ => {
            println!("No valid subcommand provided. Use --help for usage information.");
        }
    }

    Ok(())
}

fn sign_message_command(signing_key_file: &str, message: &str, output_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let signing_key = std::fs::read_to_string(signing_key_file)?;
    let signature = sign_message(&signing_key, message)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    save_key_to_file("", output_file, &signature)?;
    println!("Message signed successfully. Signature saved to: {}", output_file);
    Ok(())
}

fn validate_delegate_key_command(master_verifying_key_file: &str, delegate_certificate_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let master_verifying_key = std::fs::read_to_string(master_verifying_key_file)?;
    let delegate_certificate = std::fs::read_to_string(delegate_certificate_file)?;
    
    match validate_delegate_key(&master_verifying_key, &delegate_certificate) {
        Ok(attributes) => {
            println!("Delegate key certificate is {}.", "valid".green());
            println!("Attributes: {}", attributes);
            Ok(())
        },
        Err(e) => {
            println!("Failed to validate delegate key certificate: {}", e);
            Err(Box::new(e))
        }
    }
}

fn generate_and_save_master_key(output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (private_key, public_key) = generate_master_key().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    save_key_to_file(output_dir, "server_master_signing_key.pem", &private_key)?;
    save_key_to_file(output_dir, "server_master_verifying_key.pem", &public_key)?;
    println!("SERVER_MASTER_PRIVATE_KEY and SERVER_MASTER_VERIFYING_KEY generated successfully.");
    Ok(())
}

fn generate_and_save_delegate_key(master_key_file: &str, attributes: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let master_signing_key = std::fs::read_to_string(master_key_file)?;
    let (delegate_signing_key, delegate_certificate) = generate_delegate_key(&master_signing_key, attributes)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    save_key_to_file(output_dir, "delegate_signing_key.pem", &delegate_signing_key)?;
    save_key_to_file(output_dir, "delegate_certificate.pem", &delegate_certificate)?;
    println!("Delegate signing key and certificate generated successfully.");
    Ok(())
}

fn generate_and_save_signing_key(output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (signing_key, verifying_key) = generate_signing_key().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    save_key_to_file(output_dir, "server_signing_key.pem", &signing_key)?;
    save_key_to_file(output_dir, "server_public_key.pem", &verifying_key)?;
    println!("SERVER_SIGNING_KEY and public key generated successfully.");
    Ok(())
}

fn save_key_to_file(output_dir: &str, filename: &str, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    create_dir_all(output_dir)?;
    let file_path = Path::new(output_dir).join(filename);
    let mut file = File::create(file_path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}
