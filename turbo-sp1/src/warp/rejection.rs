use serde_json::json;
use std::convert::Infallible;
use warp::http::StatusCode;
use warp::reject::Rejection;
use warp::reply::Reply;

#[derive(Debug)]
pub struct ServerError {
    message: String,
    status_code: u16,
}

impl warp::reject::Reject for ServerError {}

impl ServerError {
    pub fn new(message: String, status_code: u16) -> Self {
        Self {
            message,
            status_code,
        }
    }

    pub fn internal_server_error(message: String) -> Rejection {
        warp::reject::custom(Self::new(message, 500))
    }

    pub fn bad_request(message: String) -> Rejection {
        warp::reject::custom(Self::new(message, 400))
    }
}

// The handle_rejection function inspects the Rejection and converts it to a JSON response.
pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    // Default error message and status code
    let (code, message) = if let Some(server_error) = err.find::<ServerError>() {
        // If the error is our custom ServerError, extract its message and status code
        (
            StatusCode::from_u16(server_error.status_code)
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            server_error.message.clone(),
        )
    } else if err.is_not_found() {
        // If the route was not found, return a 404 not found
        (StatusCode::NOT_FOUND, "Not Found".to_string())
    } else {
        // For any other errors, log the error and return a 500 internal server error
        eprintln!("Unhandled rejection: {:?}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error".to_string(),
        )
    };

    // Create a JSON body containing the error message.
    let json_body = warp::reply::json(&json!({ "message": message }));

    // Return the response with the appropriate status code.
    Ok(warp::reply::with_status(json_body, code))
}
