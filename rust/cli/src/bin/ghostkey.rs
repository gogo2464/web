use std::error::Error;
use clap::{Command, Arg, ArgAction};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::os::unix::fs::PermissionsExt;
use ghostkey::crypto::master_key::{generate_master_key, generate_master_verifying_key};
use colored::Colorize;
use log::{error, info};
use ghostkey::crypto::ghost_key::{generate_ghostkey, validate_armored_ghost_key_command};
use ghostkey::crypto::signature::{sign_message, verify_signature};
use ghostkey::crypto::generate_delegate::generate_delegate_key;
use ghostkey::crypto::validate_delegate_key;

fn main() {
    let result = run();
    if let Err(err) = result {
        eprintln!("{} {}", "Error:".red(), err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("Freenet Ghost Key Utility")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Performs various ghost key-related tasks")
        .subcommand(Command::new("sign-message")
            .about("Signs a message using a signing key and outputs the signature")
            .arg(Arg::new("signing-key-file")
                .long("signing-key-file")
                .help("The file containing the signing key (master or delegate)")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("ignore-permissions")
                .long("ignore-permissions")
                .help("Ignore file permission checks")
                .action(ArgAction::SetTrue))
            .arg(Arg::new("message")
                .long("message")
                .help("The message to sign (required if --message-file is not provided)")
                .required_unless_present("message-file")
                .conflicts_with("message-file")
                .value_name("STRING"))
            .arg(Arg::new("message-file")
                .long("message-file")
                .help("The file containing the message to sign (required if --message is not provided)")
                .required_unless_present("message")
                .conflicts_with("message")
                .value_name("FILE"))
            .arg(Arg::new("output-file")
                .long("output-file")
                .help("The file to output the signature (if omitted, signature is sent to stdout)")
                .required(false)
                .value_name("FILE")))
        .subcommand(Command::new("verify-signature")
            .about("Verifies a signature for a message using a verifying key")
            .arg(Arg::new("verifying-key-file")
                .long("verifying-key-file")
                .help("The file containing the verifying key (master or delegate)")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("message")
                .long("message")
                .help("The message to verify (required if --message-file is not provided)")
                .required_unless_present("message-file")
                .conflicts_with("message-file")
                .value_name("STRING"))
            .arg(Arg::new("message-file")
                .long("message-file")
                .help("The file containing the message to verify (required if --message is not provided)")
                .required_unless_present("message")
                .conflicts_with("message")
                .value_name("FILE"))
            .arg(Arg::new("signature-file")
                .long("signature-file")
                .help("The file containing the signature to verify")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("master-verifying-key-file")
                .long("master-verifying-key-file")
                .help("The file containing the master verifying key (optional, for delegate key validation)")
                .required(false)
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
            .arg(Arg::new("master-signing-key-file")
                .long("master-signing-key-file")
                .help("The file containing the master signing key")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("info")
                .long("info")
                .help("The info string to be included in the delegate key certificate")
                .required(true)
                .value_name("STRING"))
            .arg(Arg::new("output-dir")
                .long("output-dir")
                .help("The directory to output the delegate keys and certificate")
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
        .subcommand(Command::new("generate-verifying-key")
            .about("Generates a verifying key from a master signing key")
            .arg(Arg::new("master-signing-key-file")
                .long("master-signing-key-file")
                .help("The file containing the master signing key")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("output-file")
                .long("output-file")
                .help("The file to output the master verifying key")
                .required(true)
                .value_name("FILE")))
        .subcommand(Command::new("generate-ghost-key")
            .about("Generates a ghost key from a delegate signing key")
            .arg(Arg::new("delegate-dir")
                .long("delegate-dir")
                .help("The directory containing the delegate certificate and signing key")
                .required(true)
                .value_name("DIR"))
            .arg(Arg::new("output-dir")
                .long("output-dir")
                .help("The directory to output the ghost key files")
                .required(true)
                .value_name("DIR"))
            .arg(Arg::new("overwrite")
                .long("overwrite")
                .help("Overwrite existing ghost key file if it exists")
                .action(ArgAction::SetTrue)))
        .subcommand(Command::new("validate-ghost-key")
            .about("Validates a ghost key certificate using the master verifying key")
            .arg(Arg::new("master-verifying-key-file")
                .long("master-verifying-key-file")
                .help("The file containing the master verifying key")
                .required(true)
                .value_name("FILE"))
            .arg(Arg::new("ghost-certificate-file")
                .long("ghost-certificate-file")
                .help("The file containing the ghost key certificate")
                .required(true)
                .value_name("FILE")))
        .get_matches();

    match matches.subcommand() {
        Some(("generate-master-key", sub_matches)) => {
            let output_dir = sub_matches.get_one::<String>("output-dir").unwrap();
            generate_and_save_master_key(output_dir)?;
        }
        Some(("generate-delegate-key", sub_matches)) => {
            let master_signing_key_file = sub_matches.get_one::<String>("master-signing-key-file").unwrap();
            let info = sub_matches.get_one::<String>("info").unwrap();
            let output_dir = sub_matches.get_one::<String>("output-dir").unwrap();
            generate_and_save_delegate_key(master_signing_key_file, info, output_dir)?;
        }
        Some(("validate-delegate-key", sub_matches)) => {
            let master_verifying_key_file = sub_matches.get_one::<String>("master-verifying-key-file").unwrap();
            let delegate_certificate_file = sub_matches.get_one::<String>("delegate-certificate-file").unwrap();
            validate_delegate_key_command(master_verifying_key_file, delegate_certificate_file)?;
        }
        Some(("generate-verifying-key", sub_matches)) => {
            let master_signing_key_file = sub_matches.get_one::<String>("master-signing-key-file").unwrap();
            let output_file = sub_matches.get_one::<String>("output-file").unwrap();
            generate_master_verifying_key_command(master_signing_key_file, output_file)?;
        }
        Some(("generate-ghost-key", sub_matches)) => {
            let delegate_dir = sub_matches.get_one::<String>("delegate-dir").unwrap();
            let output_dir = sub_matches.get_one::<String>("output-dir").unwrap();
            let overwrite = sub_matches.get_flag("overwrite");
            generate_ghostkey_command(delegate_dir, output_dir, overwrite)?;
        }
        Some(("validate-ghost-key", sub_matches)) => {
            let master_verifying_key_file = sub_matches.get_one::<String>("master-verifying-key-file").unwrap();
            let ghost_certificate_file = sub_matches.get_one::<String>("ghost-certificate-file").unwrap();
            validate_ghost_key_command(master_verifying_key_file, ghost_certificate_file)?;
        }
        Some(("sign-message", sub_matches)) => {
            let signing_key_file = sub_matches.get_one::<String>("signing-key-file").unwrap();
            let message = sub_matches.get_one::<String>("message");
            let message_file = sub_matches.get_one::<String>("message-file");
            let output_file = sub_matches.get_one::<String>("output-file");
            let ignore_permissions = sub_matches.get_flag("ignore-permissions");
            sign_message_command(signing_key_file, message.map(|s| s.as_str()), message_file.map(|s| s.as_str()), output_file.map(|s| s.as_str()), ignore_permissions)?;
        }
        Some(("verify-signature", sub_matches)) => {
            let verifying_key_file = sub_matches.get_one::<String>("verifying-key-file").unwrap();
            let message = sub_matches.get_one::<String>("message");
            let message_file = sub_matches.get_one::<String>("message-file");
            let signature_file = sub_matches.get_one::<String>("signature-file").unwrap();
            let master_verifying_key_file = sub_matches.get_one::<String>("master-verifying-key-file");
            verify_signature_command(verifying_key_file, message.map(|s| s.as_str()), message_file.map(|s| s.as_str()), signature_file, master_verifying_key_file.map(|s| s.as_str()))?;
        }
        _ => {
            info!("No valid subcommand provided. Use --help for usage information.");
        }
    }

    Ok(())
}

fn check_file_permissions(file_path: &str, ignore_permissions: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !ignore_permissions {
        let metadata = std::fs::metadata(file_path)?;
        let permissions = metadata.permissions();
        let mode = permissions.mode();
        
        if mode & 0o077 != 0 {
            return Err(format!("The signing key file '{}' has incorrect permissions. It should not be readable or writable by group or others. Use chmod 600 to set the correct permissions, or use --ignore-permissions to override this check.", file_path).into());
        }
    }
    Ok(())
}

fn sign_message_command(signing_key_file: &str, message: Option<&str>, message_file: Option<&str>, output_file: Option<&str>, ignore_permissions: bool) -> Result<(), Box<dyn std::error::Error>> {
    check_file_permissions(signing_key_file, ignore_permissions)?;
    let signing_key = std::fs::read_to_string(signing_key_file)?;
    
    let message_content = if let Some(msg) = message {
        msg.to_string()
    } else if let Some(file) = message_file {
        std::fs::read_to_string(file)?
    } else {
        return Err("Either message or message-file must be provided".into());
    };

    let signature = sign_message(&signing_key, &message_content)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    match output_file {
        Some(file) => {
            save_key_to_file("", file, &signature, true)?;
            info!("Message signed successfully. Signature saved to: {}", file);
        },
        None => {
            info!("{}", signature);
        }
    }
    Ok(())
}

fn validate_delegate_key_command(master_verifying_key_file: &str, delegate_certificate_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let master_verifying_key = std::fs::read_to_string(master_verifying_key_file)
        .map_err(|e| {
            error!("Failed to read master verifying key file: {}", e);
            format!("Failed to read master verifying key file: {}", e)
        })?;
    
    let delegate_certificate = match std::fs::read_to_string(delegate_certificate_file) {
        Ok(content) => content,
        Err(e) => {
            error!("Failed to read delegate certificate file: {}", e);
            return Err(format!("Failed to read delegate certificate file: {}", e).into());
        }
    };
    
    match validate_delegate_key(&master_verifying_key, &delegate_certificate) {
        Ok(delegate_info) => {
            println!("Delegate certificate is {}.", "valid".green());
            println!("Delegate info: {}", delegate_info);
            Ok(())
        }
        Err(e) => {
            error!("Delegate certificate is {}.", "invalid".red());
            error!("Error: {}", e);
            Err(format!("Delegate certificate validation failed: {}", e).into())
        }
    }
}

fn generate_and_save_master_key(output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let signing_key_path = Path::new(output_dir).join("master_signing_key.pem");
    let verifying_key_path = Path::new(output_dir).join("master_verifying_key.pem");

    if signing_key_path.exists() || verifying_key_path.exists() {
        return Err(format!("One or both of the files '{}' or '{}' already exist. Please choose a different output directory or remove the existing files.", signing_key_path.display(), verifying_key_path.display()).into());
    }

    let (private_key, public_key) = generate_master_key().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    save_key_to_file(output_dir, "master_signing_key.pem", &private_key, true)?;
    save_key_to_file(output_dir, "master_verifying_key.pem", &public_key, false)?;
    println!("{}", "MASTER_SIGNING_KEY and MASTER_VERIFYING_KEY generated successfully.".green());
    println!("Files created:");
    println!("  Master signing key: {}", signing_key_path.display());
    println!("  Master verifying key: {}", verifying_key_path.display());
    Ok(())
}

fn generate_and_save_delegate_key(master_key_file: &str, info: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Create the output directory if it doesn't exist
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory '{}': {}", output_dir, e))?;

    // Check if the master key file exists
    if !std::path::Path::new(master_key_file).exists() {
        return Err(format!("Master signing key file '{}' not found", master_key_file).into());
    }

    check_file_permissions(master_key_file, false)?;
    let master_signing_key = std::fs::read_to_string(master_key_file)
        .map_err(|e| format!("Failed to read master signing key file '{}': {}", master_key_file, e))?;
    let (delegate_certificate, delegate_signing_key) = generate_delegate_key(&master_signing_key, info)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    let cert_path = Path::new(output_dir).join("delegate_certificate.pem");
    let key_path = Path::new(output_dir).join("delegate_signing_key.pem");
    
    if cert_path.exists() || key_path.exists() {
        return Err(format!("One or both of the files '{}' or '{}' already exist. Please choose a different output directory or remove the existing files.", cert_path.display(), key_path.display()).into());
    }
    
    save_key_to_file(output_dir, "delegate_certificate.pem", &delegate_certificate, false)?;
    save_key_to_file(output_dir, "delegate_signing_key.pem", &delegate_signing_key, true)?;
    
    println!("{}", "Delegate certificate and signing key generated successfully.".green());
    println!("Files created:");
    println!("  Delegate certificate: {}", cert_path.display());
    println!("  Delegate signing key: {}", key_path.display());
    Ok(())
}

fn save_key_to_file(output_dir: &str, filename: &str, content: &str, is_private: bool) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let file_path = Path::new(output_dir).join(filename);
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = File::create(&file_path)
        .map_err(|e| format!("Failed to create file '{}': {}", file_path.display(), e))?;
    file.write_all(content.as_bytes())?;
    
    if is_private {
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o600);
        file.set_permissions(perms)?;
    }
    
    Ok(file_path)
}
fn verify_signature_command(verifying_key_file: &str, message: Option<&str>, message_file: Option<&str>, signature_file: &str, master_verifying_key_file: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let verifying_key = std::fs::read_to_string(verifying_key_file)?;
    let signature = std::fs::read_to_string(signature_file)?;
    
    let message_content = if let Some(msg) = message {
        msg.to_string()
    } else if let Some(file) = message_file {
        std::fs::read_to_string(file)?
    } else {
        return Err("Either message or message-file must be provided".into());
    };

    if let Some(master_key_file) = master_verifying_key_file {
        let master_verifying_key = std::fs::read_to_string(master_key_file)?;
        validate_delegate_key(&master_verifying_key, &verifying_key)?;
        println!("Delegate key validated successfully.");
    }

    match verify_signature(&verifying_key, &message_content, &signature) {
        Ok(true) => {
            info!("Signature is {}.", "valid".green());
            Ok(())
        },
        Ok(false) => {
            info!("Signature is {}.", "invalid".red());
            Ok(())
        },
        Err(e) => {
            error!("Failed to verify signature: {}", e);
            Err(Box::new(e))
        }
    }
}

fn generate_master_verifying_key_command(master_signing_key_file: &str, output_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let master_signing_key = std::fs::read_to_string(master_signing_key_file)?;
    let master_verifying_key = generate_master_verifying_key(&master_signing_key)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    save_key_to_file("", output_file, &master_verifying_key, false)?;
    
    println!("Server Master Verifying Key generated successfully.");
    println!("File created: {}", output_file);
    Ok(())
}

fn generate_ghostkey_command(delegate_dir: &str, output_dir: &str, overwrite: bool) -> Result<(), Box<dyn std::error::Error>> {
    info!("Reading delegate certificate and signing key from directory: {}", delegate_dir);
    let delegate_certificate = std::fs::read_to_string(Path::new(delegate_dir).join("delegate_certificate.pem"))
        .map_err(|e| {
            error!("{}", format!("Failed to read delegate certificate file: {}", e).red());
            format!("Failed to read delegate certificate file: {}", e)
        })?;
    let delegate_signing_key = std::fs::read_to_string(Path::new(delegate_dir).join("delegate_signing_key.pem"))
        .map_err(|e| {
            error!("{}", format!("Failed to read delegate signing key file: {}", e).red());
            format!("Failed to read delegate signing key file: {}", e)
        })?;
    
    info!("Generating ghost key from delegate certificate and signing key");
    let ghostkey_certificate = generate_ghostkey(&delegate_certificate, &delegate_signing_key)
        .map_err(|e| {
            error!("{}", format!("Failed to generate ghostkey: {}", e).red());
            format!("Failed to generate ghostkey: {}", e)
        })?;
    
    let file_path = Path::new(output_dir).join("ghostkey_certificate.pem");
    if file_path.exists() && !overwrite {
        error!("{}", format!("File '{}' already exists", file_path.display()).red());
        return Err(format!("File '{}' already exists. Use --overwrite to replace the existing file or choose a different output directory.", file_path.display()).into());
    }
    
    info!("Saving ghost key certificate to file: {}", file_path.display());
    match save_key_to_file(output_dir, "ghostkey_certificate.pem", &ghostkey_certificate, true) {
        Ok(_) => {
            println!("{}", "Ghost key generated and saved successfully.".green());
            println!("File created: {}", file_path.display());
            Ok(())
        },
        Err(e) => {
            error!("{}", format!("Failed to save ghostkey certificate: {}", e).red());
            Err(e.into())
        }
    }
}

fn validate_ghost_key_command(master_verifying_key_file: &str, ghost_certificate_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let master_verifying_key = std::fs::read_to_string(master_verifying_key_file)?;
    let ghost_certificate = std::fs::read_to_string(ghost_certificate_file)?;

    match validate_armored_ghost_key_command(&master_verifying_key, &ghost_certificate, ghost_certificate_file) {
        Ok(_) => {
            info!("Ghost key certificate is {}.", "valid".green());
            Ok(())
        }
        Err(e) => {
            error!("Ghost key certificate is {}.", "invalid".red());
            error!("Reason: {}", e);
            Err(Box::new(e))
        }
    }
}
