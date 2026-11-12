//! # Rocket.rs some-stripe Full Service Binary Entrypoint
//!
//! Launches the Rocket web server configured from `.env` and environment variables.

use rocket_stripe_example::rocket;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let _rocket = rocket().launch().await?;
    Ok(())
}
