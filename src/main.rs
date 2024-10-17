use std::{convert::Infallible, path::PathBuf};

use axum::{http::{Request, Uri}, response::Html, routing::get, Router};
use tower::ServiceBuilder;
use tracing::info;
use std::net::SocketAddr;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use clap::Parser;

const DEFAULT_PORT: u16 = 8015;

#[derive(Debug, Parser)]
struct CliArgs {
    /// The directory to serve
    serve_dir: PathBuf,
    /// The port to listnen on
    #[arg(short, long, default_value_t=DEFAULT_PORT)]
    port: u16,
}

fn rewrite_url<B>(mut request: Request<B>) -> Request<B> {
    let uri = request.uri().path().to_string();

    if !uri.contains('.') {
        let new_path = format!("{}.html", uri);
        let new_uri = Uri::builder()
            .path_and_query(&new_path)
            .build()
            .unwrap();

        info!("Rewriting request uri {} to {}", uri, new_uri);
        *request.uri_mut() = new_uri;
    }

    request
}

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=info,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let serve_dir = ServeDir::new(args.serve_dir.clone());

    //let app: Router = Router::new().nest_service("/", ServeDir::new(args.serve_dir.clone()));
    let app = Router::new()
        .nest_service(
            "/",
            ServiceBuilder::new()
            .map_request(rewrite_url)
            .service(serve_dir)
        ).layer(TraceLayer::new_for_http());

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
