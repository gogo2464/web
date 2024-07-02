#[macro_use] extern crate rocket;

use dotenv::dotenv;
mod routes;
mod stripe_handler;

use rocket::fairing::AdHoc;
use rocket::shield::{Shield, XssFilter, Referrer};
use rocket::fairing::AdHoc;

#[launch]
fn rocket() -> _ {
    dotenv().ok();
    rocket::build()
        .attach(routes::CORS)
        .attach(routes::RequestTimer)
        .attach(AdHoc::on_response("Powered-By Header", |_, res| Box::pin(async move {
            res.set_raw_header("X-Powered-By", "Freenet Rocket API");
        })))
        .attach(Shield::new()
            .enable(XssFilter::EnableBlock)
            .enable(Referrer::NoReferrer)
            .enable(Referrer::StrictOriginWhenCrossOrigin))
        .attach(AdHoc::on_response("Content-Type-Options Header", |_, res| Box::pin(async move {
            res.set_raw_header("X-Content-Type-Options", "nosniff");
        })))
        .mount("/", routes::routes())
}
