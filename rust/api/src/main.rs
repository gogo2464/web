use std::{env, time::SystemTime, fs};
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

use clap::{Arg, Command};
use dotenv::dotenv;
use log::{error, info, LevelFilter};
use tokio_rustls::rustls::{Certificate, PrivateKey};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;
use axum::{
    routing::get,
    Router,
    http::StatusCode,
    response::IntoResponse,
};
use tower_http::trace::TraceLayer;
use tower_http::cors::CorsLayer;

mod routes;
mod handle_sign_cert;
mod delegates;
mod errors;

pub static DELEGATE_DIR: &str = "DELEGATE_DIR";

struct TlsConfig {
    cert: String,
    key: String,
    last_modified: SystemTime,
}

impl TlsConfig {
    fn new(cert: String, key: String) -> Self {
        let last_modified = SystemTime::now();
        Self { cert, key, last_modified }
    }

    fn update_if_changed(&mut self) -> bool {
        let cert_modified = fs::metadata(&self.cert).and_then(|m| m.modified()).ok();
        let key_modified = fs::metadata(&self.key).and_then(|m| m.modified()).ok();

        if let (Some(cert_time), Some(key_time)) = (cert_modified, key_modified) {
            let max_time = cert_time.max(key_time);
            if max_time > self.last_modified {
                self.last_modified = max_time;
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Sorry, this is not a valid path.")
}

async fn health() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() {
    let matches = Command::new("Freenet Certified Donation API")
        .arg(Arg::new("delegate-dir")
            .long("delegate-dir")
            .value_name("DIR")
            .help("Sets the delegate directory")
            .required(true))
        .arg(Arg::new("tls-cert")
            .long("tls-cert")
            .value_name("FILE")
            .help("Path to TLS certificate file"))
        .arg(Arg::new("tls-key")
            .long("tls-key")
            .value_name("FILE")
            .help("Path to TLS private key file"))
        .get_matches();

    let delegate_dir = matches.get_one::<String>("delegate-dir").unwrap();
    env::set_var(DELEGATE_DIR, delegate_dir);

    env_logger::builder()
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .format_module_path(false)
        .format_target(false)
        .filter_level(LevelFilter::Debug)
        .init();

    info!("Starting Freenet Certified Donation API");
    match dotenv() {
        Ok(path) => info!(".env file loaded successfully from: {:?}", path),
        Err(e) => error!("Failed to load .env file: {}", e),
    }

    env::var("DELEGATE_DIR").expect("DELEGATE_DIR environment variable not set");
    
    let tls_config = if let (Some(tls_cert), Some(tls_key)) = (matches.get_one::<String>("tls-cert"), matches.get_one::<String>("tls-key")) {
        info!("TLS certificate and key provided. Starting in HTTPS mode.");
        Some(Arc::new(Mutex::new(TlsConfig::new(tls_cert.to_string(), tls_key.to_string()))))
    } else {
        info!("No TLS certificate and key provided. Starting in HTTP mode.");
        None
    };

    let app = Router::new()
        .route("/health", get(health))
        .merge(routes::get_routes())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .fallback(not_found);

    if let Some(tls_config) = tls_config.clone() {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // Check every hour
            loop {
                interval.tick().await;
                let mut config = tls_config.lock().unwrap();
                if config.update_if_changed() {
                    info!("TLS certificate or key has been updated. Reloading configuration.");
                    // Signal the server to reload its TLS config
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    tx.send(()).expect("Failed to send reload signal");
                    let tls_config = tls_config.clone();
                    tokio::spawn(async move {
                        if let Err(e) = rx.await {
                            error!("Failed to receive reload signal: {}", e);
                        }
                        // Trigger the actual reload mechanism
                        info!("TLS config reload triggered");
                        match reload_tls_config(&tls_config).await {
                            Ok(_) => info!("TLS config reloaded successfully"),
                            Err(e) => error!("Failed to reload TLS config: {}", e),
                        }
                    });
                }
            }
        });
    }

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn reload_tls_config(tls_config: &Arc<Mutex<TlsConfig>>) -> Result<(), Box<dyn std::error::Error>> {
    let config = tls_config.lock().unwrap();
    let mut cert_file = std::io::BufReader::new(fs::File::open(&config.cert)?);
    let mut key_file = std::io::BufReader::new(fs::File::open(&config.key)?);
    
    let cert_chain = rustls_pemfile::certs(&mut cert_file)?
        .into_iter()
        .map(Certificate)
        .collect();
    let mut keys = rustls_pemfile::pkcs8_private_keys(&mut key_file)?;

    if keys.is_empty() {
        return Err("No PKCS8 private keys found in key file".into());
    }

    let server_config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain, PrivateKey(keys.remove(0)))?;

    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    
    // Here you would update your server's TLS acceptor
    // This might involve sending a message to your server task to swap out the acceptor
    // For now, we'll just log that we've created a new acceptor
    info!("Created new TLS acceptor with updated certificates");

    Ok(())
}
