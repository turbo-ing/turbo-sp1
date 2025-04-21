use futures_util::FutureExt;
use serde_json::json;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use warp::http::StatusCode;
use warp::reject::Rejection;
use warp::reply::Reply;

type HandlerResult = Result<Box<dyn Reply>, Rejection>;
type HandlerFuture = Pin<Box<dyn Future<Output = HandlerResult> + Send>>;

pub fn safe_handler<F, Args>(f: F) -> impl Fn(Args) -> HandlerFuture
where
    F: Fn(Args) -> HandlerFuture + Send + Sync + 'static,
    Args: Send + 'static,
{
    move |args| {
        let fut = f(args);
        Box::pin(async move {
            match AssertUnwindSafe(fut).catch_unwind().await {
                Ok(Ok(reply)) => Ok(reply),
                Ok(Err(rej)) => Err(rej),
                Err(_) => Ok(Box::new(warp::reply::with_status(
                    warp::reply::json(&json!({"message": "Internal server error"})),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )) as Box<dyn Reply>),
            }
        })
    }
}
