use std::io;
use hyper::StatusCode;
use tower_http::services::ServeDir;

use axum::{
    routing::{MethodRouter, get_service},
    response::{IntoResponse}
};


async fn handle_error(_err: io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}

pub fn create_static_service(static_path: String) -> MethodRouter {
    get_service(ServeDir::new(static_path)).handle_error(handle_error)
}
