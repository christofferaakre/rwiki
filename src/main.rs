pub mod routes;

use std::{convert::Infallible, path::PathBuf, sync::{Arc, LazyLock, Mutex}};

use axum::{
    body::Bytes,
    http::{HeaderValue, Request, Response, Uri},
    response::Html,
    routing::get,
    Router,
};
use hyper::body::Body;
use std::net::SocketAddr;
use tower::{ServiceBuilder, ServiceExt};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use http_body_util::{combinators::BoxBody, BodyExt};

use clap::Parser;

use routes::get_router;

const DEFAULT_PORT: u16 = 8015;

#[derive(Debug, Parser)]
struct CliArgs {
    /// The directory to serve
    serve_dir: PathBuf,
    /// The port to listnen on
    #[arg(short, long, default_value_t=DEFAULT_PORT)]
    port: u16,
}

static ROOT_PATH: LazyLock<Arc<Mutex<Option<PathBuf>>>> = LazyLock::new(|| {
    Arc::new(Mutex::new(None))
});


#[tokio::main]
async fn main() {
    let args = CliArgs::parse();
    *ROOT_PATH.lock().unwrap() = Some(args.serve_dir.clone());

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=info", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = get_router()
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!(
        "Listening on {}, serving dir {}",
        listener.local_addr().unwrap(),
        args.serve_dir.display()
    );
    axum::serve(listener, app.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();
}

