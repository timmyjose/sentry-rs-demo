use std::env;

use actix_web::{
    get, http::header::ContentType, post, web, App, HttpResponse, HttpServer, Responder,
    ResponseError,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Actix(#[from] actix_web::Error),

    #[error("cannot divide by zero")]
    DivideByZero,

    #[error(transparent)]
    DotEnvy(#[from] dotenvy::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("SENRTY_DSN is unset")]
    MissingSentryDsn,
}

impl ResponseError for Error {}

pub type Result<T> = std::result::Result<T, Error>;

async fn add(x: i32, y: i32) -> Result<i32> {
    Ok(x + y)
}

async fn sub(x: i32, y: i32) -> Result<i32> {
    Ok(x - y)
}

async fn mul(x: i32, y: i32) -> Result<i32> {
    Ok(x * y)
}

async fn div(x: i32, y: i32) -> Result<i32> {
    if y == 0 {
        Err(Error::DivideByZero)
    } else {
        Ok(x + y)
    }
}

async fn init_sentry() -> Result<()> {
    let sentry_dsn = env::var("SENTRY_DSN").map_err(|_| Error::MissingSentryDsn)?;

    let _guard = sentry::init((
        sentry_dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct CalculationRequest {
    x: i32,
    y: i32,
}

#[derive(Debug, Serialize)]
pub struct CalculationResponse {
    res: i32,
}

#[post("/add")]
async fn handle_add(
    query: web::Json<CalculationRequest>,
) -> Result<web::Json<CalculationResponse>> {
    let x = query.x;
    let y = query.y;

    let sum = add(x, y).await?;
    Ok(web::Json(CalculationResponse { res: sum }))
}

#[post("/sub")]
async fn handle_sub(
    query: web::Json<CalculationRequest>,
) -> Result<web::Json<CalculationResponse>> {
    let x = query.x;
    let y = query.y;

    let diff = sub(x, y).await?;
    Ok(web::Json(CalculationResponse { res: diff }))
}

#[post("/mul")]
async fn handle_mul(
    query: web::Json<CalculationRequest>,
) -> Result<web::Json<CalculationResponse>> {
    let x = query.x;
    let y = query.y;

    let prod = mul(x, y).await?;
    Ok(web::Json(CalculationResponse { res: prod }))
}

#[post("/div")]
async fn handle_div(
    query: web::Json<CalculationRequest>,
) -> Result<web::Json<CalculationResponse>> {
    let x = query.x;
    let y = query.y;

    let quot = div(x, y).await?;
    Ok(web::Json(CalculationResponse { res: quot }))
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    status: String,
}

#[get("/status")]
async fn status() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(StatusResponse {
            status: "OK".to_string(),
        })
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;
    init_sentry().await?;

    HttpServer::new(|| {
        App::new().service(
            web::scope("/api/v0")
                .service(status)
                .service(handle_add)
                .service(handle_sub)
                .service(handle_mul)
                .service(handle_div),
        )
    })
    .bind(("127.0.0.1", 9999))?
    .run()
    .await?;

    Ok(())
}
