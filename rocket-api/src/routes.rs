use rocket::http::{Header, Status};
use rocket::{Request, Response, Data};
use rocket::fairing::{Fairing, Info, Kind};
use serde::{Serialize, Deserialize};
use rocket::serde::json::Json;
use crate::stripe_handler::{SignCertificateRequest, SignCertificateResponse, sign_certificate};
use std::time::Instant;
use stripe::{Client, PaymentIntent, CreatePaymentIntent, Currency};

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "http://localhost:1313"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

pub struct RequestTimer;

#[rocket::async_trait]
impl Fairing for RequestTimer {
    fn info(&self) -> Info {
        Info {
            name: "Request Timer",
            kind: Kind::Request | Kind::Response
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
        request.local_cache(|| Instant::now());
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, _: &mut Response<'r>) {
        let start_time = request.local_cache(|| Instant::now());
        let duration = start_time.elapsed();
        println!("Request to {} took {}ms", request.uri(), duration.as_millis());
    }
}

#[derive(Serialize, Deserialize)]
struct Message {
    content: String,
}

#[derive(Deserialize)]
struct DonationRequest {
    amount: i64,
    currency: String,
}

#[derive(Serialize)]
struct DonationResponse {
    client_secret: String,
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/message")]
fn get_message() -> Json<Message> {
    Json(Message {
        content: String::from("Welcome to the Freenet API! This message confirms that the API is functioning correctly."),
    })
}


#[post("/sign-certificate", data = "<request>")]
pub async fn sign_certificate_route(request: Json<SignCertificateRequest>) -> Result<Json<SignCertificateResponse>, (Status, String)> {
    match sign_certificate(request.into_inner()).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            eprintln!("Error signing certificate: {}", e);
            Err((Status::InternalServerError, format!("Error signing certificate: {}", e)))
        },
    }
}

#[options("/sign-certificate")]
pub fn options_sign_certificate() -> Status {
    Status::Ok
}

#[post("/create-donation", data = "<request>")]
pub async fn create_donation(request: Json<DonationRequest>) -> Result<Json<DonationResponse>, (Status, String)> {
    let secret_key = std::env::var("STRIPE_SECRET_KEY").expect("Missing STRIPE_SECRET_KEY in env");
    let client = Client::new(secret_key);

    let currency = match request.currency.as_str() {
        "usd" => Currency::USD,
        "eur" => Currency::EUR,
        "gbp" => Currency::GBP,
        _ => return Err((Status::BadRequest, "Invalid currency".to_string())),
    };

    let params = CreatePaymentIntent {
        amount: request.amount,
        currency,
        automatic_payment_methods: Some(stripe::CreatePaymentIntentAutomaticPaymentMethods {
            enabled: true,
            allow_redirects: None,
        }),
        payment_method_types: Some(vec!["card".to_string()]),
        expand: &[],
        application_fee_amount: None,
        capture_method: None,
        confirm: None,
        confirmation_method: None,
        customer: None,
        description: None,
        error_on_requires_action: None,
        mandate: None,
        mandate_data: None,
        metadata: None,
        off_session: None,
        on_behalf_of: None,
        payment_method: None,
        payment_method_configuration: None,
        payment_method_data: None,
        payment_method_options: None,
        radar_options: None,
        receipt_email: None,
        return_url: None,
        setup_future_usage: None,
        shipping: None,
        statement_descriptor: None,
        statement_descriptor_suffix: None,
        transfer_data: None,
        transfer_group: None,
        use_stripe_sdk: None,
    };

    match PaymentIntent::create(&client, params).await {
        Ok(intent) => Ok(Json(DonationResponse {
            client_secret: intent.client_secret.unwrap(),
        })),
        Err(e) => Err((Status::InternalServerError, format!("Failed to create payment intent: {}", e))),
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![index, get_message, sign_certificate_route, options_sign_certificate, create_donation]
}
