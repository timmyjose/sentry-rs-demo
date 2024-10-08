use std::{env, sync::Arc};

use actix_cors::Cors;
use actix_web::{
    get,
    http::{header::ContentType, StatusCode},
    post, web, App, HttpResponse, HttpServer, Responder, ResponseError,
};
use middleware::Middleware;
use sentry::ClientInitGuard;
use sentry_tracing::EventFilter;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

mod middleware;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot divide by zero")]
    DivideByZero,

    #[error("SENRTY_DSN is unset")]
    MissingSentryDsn,

    #[error(transparent)]
    Actix(#[from] actix_web::Error),

    #[error(transparent)]
    DotEnvy(#[from] dotenvy::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct HTTPError {
    pub status_code: StatusCode,
    pub source: Box<dyn std::error::Error>,
}

impl std::error::Error for HTTPError {}

impl std::fmt::Display for HTTPError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ {}, {} }}", self.status_code, self.source)
    }
}

impl From<Error> for HTTPError {
    fn from(err: Error) -> Self {
        let http_error = match err {
            Error::DivideByZero => HTTPError {
                status_code: StatusCode::BAD_REQUEST,
                source: err.into(),
            },
            _ => HTTPError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                source: err.into(),
            },
        };

        sentry::with_scope(
            |scope| {
                scope.set_extra("status_code", http_error.status_code.as_u16().into());
            },
            || sentry::capture_error(&http_error),
        );

        http_error
    }
}

impl ResponseError for HTTPError {}

pub type HttpResult<T> = std::result::Result<T, HTTPError>;

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

async fn init_tracing() -> Result<ClientInitGuard> {
    let sentry_dsn = env::var("SENTRY_DSN").map_err(|_| Error::MissingSentryDsn)?;
    let _guard = sentry::init((
        sentry_dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            before_send: Some(Arc::new(|event| {
                if let Some(status_code) = event.extra.get("status_code") {
                    let status_code = status_code.as_u64().unwrap_or(200);

                    if (400..500).contains(&status_code) {
                        println!("Intercepted a 4xx notification");
                        return None;
                    }
                }
                Some(event)
            })),
            ..Default::default()
        },
    ));

    let sentry_layer = sentry_tracing::layer().event_filter(|md| match md.level() {
        &tracing::Level::ERROR => EventFilter::Event,
        _ => EventFilter::Ignore,
    });

    let log_level_filter = EnvFilter::new("INFO");
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(sentry_layer)
        .with(log_level_filter)
        .init();

    Ok(_guard)
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

#[tracing::instrument]
#[post("/add")]
async fn handle_add(
    body: web::Json<CalculationRequest>,
) -> HttpResult<web::Json<CalculationResponse>> {
    info!(method = "handle_add", ?body, "adding two numbers together");
    error!("add");

    let x = body.x;
    let y = body.y;

    let sum = add(x, y).await?;
    Ok(web::Json(CalculationResponse { res: sum }))
}

#[tracing::instrument]
#[post("/sub")]
async fn handle_sub(
    body: web::Json<CalculationRequest>,
) -> HttpResult<web::Json<CalculationResponse>> {
    info!(
        method = "handle_sub",
        ?body,
        "subtracting a number from another"
    );

    let x = body.x;
    let y = body.y;

    let diff = sub(x, y).await?;
    Ok(web::Json(CalculationResponse { res: diff }))
}

#[tracing::instrument]
#[post("/mul")]
async fn handle_mul(
    body: web::Json<CalculationRequest>,
) -> HttpResult<web::Json<CalculationResponse>> {
    info!(method = "handle_mul", ?body, "multiplying two numbers");

    let x = body.x;
    let y = body.y;

    let prod = mul(x, y).await?;
    Ok(web::Json(CalculationResponse { res: prod }))
}

#[tracing::instrument]
#[post("/div")]
async fn handle_div(
    body: web::Json<CalculationRequest>,
) -> HttpResult<web::Json<CalculationResponse>> {
    info!(method = "handle_div", ?body, "Dividing a number by another");

    let x = body.x;
    let y = body.y;

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

    let _guard = init_tracing().await?;

    HttpServer::new(|| {
        let cors = Cors::permissive();
        App::new().wrap(cors).wrap(Middleware).service(
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
