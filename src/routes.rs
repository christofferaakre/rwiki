use std::{path::PathBuf, sync::Arc};

use axum_macros::{debug_handler, debug_middleware};
use axum::{
    extract::{path::ErrorKind, Path},
    response::{Html, Response},
    routing::get,
    Router,
};
use hyper::StatusCode;
use std::sync::{LazyLock, Mutex};
use tracing::info;

use crate::ROOT_PATH;

fn get_header() -> &'static str {
    include_str!("../templates/header.html")
}

fn get_footer() -> &'static str {
    include_str!("../templates/footer.html")
}

fn fill_template(content: String) -> String {
    let template = include_str!("../templates/page.html");
   template
       .replace("##HEADER##", get_header())
       .replace("##CONTENT##", &content)
       .replace("##FOOTER##", get_footer())
   
}

fn not_found() -> (StatusCode, Html<String>) {
    (StatusCode::OK, Html::from(String::from("404 - Not Found")))
}

fn root_path() -> PathBuf {
    ROOT_PATH.lock().unwrap().clone().unwrap()
}

pub fn relative_path(path: impl AsRef<std::path::Path>, relative_to: impl AsRef<std::path::Path>) -> PathBuf {
    path.as_ref()
        .canonicalize()
        .unwrap()
        .strip_prefix(relative_to.as_ref())
        .unwrap()
        .into()
}

async fn list_index() -> (StatusCode, Html<String>) {
    directory_listing(root_path()).await
}

async fn directory_listing(path: impl AsRef<std::path::Path>) -> (StatusCode, Html<String>) {
    let listing = std::fs::read_dir(path.as_ref()).unwrap().filter_map(|entry| {
        if let Ok(entry) = entry {
            if entry.path().is_dir() || entry.path().extension().unwrap().to_str() == Some("html") {
                Some(entry)
            } else {
                None
            }
        } else {
            None
        }
    });
    let mut output = String::new();
    output.push_str("<ul>");
    for item in listing {
        let file = item.path().to_str().unwrap().to_owned();
        let relative = relative_path(&file, &path).to_str().unwrap().to_string();
        let name = relative.replace(".html", "");
        output.push_str(&format!(
            "
               <li><a href=\"{}\">{}</a></li>
               ",
            name, name
        ));
    }
    output.push_str("</ul>");

    (StatusCode::OK, Html::from(output))
}

async fn serve_html(Path(file): Path<PathBuf>) -> (StatusCode, Html<String>) {
    let mut path = root_path().join(file);
    info!("Request for {}", path.display());

    if !path.exists() {
        let filename = format!("{}.html", path.file_name().unwrap().to_str().unwrap());
        let html_path = path.parent().unwrap().join(filename);
        if html_path.exists() {
            path = html_path;
        } else {
            return not_found();
        }
    }

    let normalise_result = path.canonicalize();
    let normalised = match normalise_result {
        Ok(normalised) => normalised,
        Err(err) => match err.kind() {
            std::io::ErrorKind::NotFound => return not_found(),
            err => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html::from(err.to_string()),
                )
            }
        },
    };

    if path.is_dir() {
        return directory_listing(path).await;
    }

    let root_normalised = root_path().canonicalize().unwrap();
    let _ = normalised
        .strip_prefix(root_normalised)
        .expect("Requested file path is outside root directory");

    let content = std::fs::read_to_string(path).unwrap();
    let content = fill_template(content);

    (StatusCode::OK, Html::from(content))
}

const TEXT_CSS: &str = "text/css";
#[axum_macros::debug_handler]
async fn serve_style_css() -> Response<String> {
    let body = include_str!("../static/style.css");
    Response::builder()
        .status(StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, TEXT_CSS)
        .body(String::from(body))
        .unwrap()
}


pub fn get_router() -> Router {
    Router::new()
        .route("/", get(list_index))
        .route("/:file", get(serve_html))
        .route("/style.css", get(serve_style_css))
}
