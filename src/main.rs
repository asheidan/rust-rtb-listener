use std::task::Context;
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;
use chrono::prelude::Utc;
use core_affinity;
use futures::task::Poll;
use futures::Future;
use hyper::server::conn::AddrStream;
use hyper::service::Service;
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};
use redis::AsyncCommands;
use redis::RedisError;

async fn redis_connect() -> redis::aio::ConnectionManager {
    //let nodes = vec!["redis://192.168.0.134/"];
    //
    let connection_info = "redis://192.168.0.134/";

    println!("Connecting to redis: {}", connection_info);

    redis::Client::open(connection_info).unwrap().get_tokio_connection_manager().await.unwrap()
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

#[tokio::main(core_threads = 1)]
async fn main() -> std::io::Result<()> {
    core_affinity::set_for_current(core_affinity::CoreId { id: 0 });

    let address = SocketAddr::from(([0, 0, 0, 0], 8080));

    let redis_client = redis_connect().await;

    let server = Server::bind(&address)
        .http1_only(true)
        .http1_keepalive(true)
        .tcp_keepalive(Some(Duration::new(150, 0)))
        .serve(MakeSvc { redis: redis_client });

    println!("Listening on {}", server.local_addr());

    let graceful = server.with_graceful_shutdown(shutdown_signal());

    if let Err(e) = graceful.await {
        eprintln!("server error: {}", e);
    }

    return Ok(());
}

trait Rtb {
    type Future;

    fn handle_ready(&self) -> Self::Future;

    fn handle_404(&self) -> Self::Future;

    fn handle_category(&self, url: String) -> Self::Future;

    fn handle_missing(&self) -> Self::Future;
}


struct RtbService {
    redis: redis::aio::ConnectionManager,
}
impl Rtb for RtbService {
    type Future = Pin<Box<dyn Future<Output = Result<Response<Body>, hyper::Error>> + Send>>;

    fn handle_ready(&self) -> Self::Future {
        Box::pin(async { Ok(Response::new(Body::from("1\n"))) })
    }

    fn handle_404(&self) -> Self::Future {
        Box::pin(async {
            Ok(
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("\n"))
                    .unwrap()
            )
        })
    }

    fn handle_category(&self, url: String) -> Self::Future {
        let mut redis = self.redis.clone();

        let fut = async move {
            let result: Result<String, RedisError> = redis.get(url).await;

            let mut segments: String = match result {
                Ok(data) => data,
                _ => "".to_string(),
            };

            segments.push('\n');

            Ok(Response::new(Body::from(segments)))
        };

        Box::pin(fut)
    }

    fn handle_missing(&self) -> Self::Future {
        Box::pin(async { Ok(Response::new(Body::from("\n"))) })
    }
}

impl Service<Request<Body>> for RtbService {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {

        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {

        let handler = match (request.method(), request.uri().path()) {
            (&Method::GET, "/category") => {
                if let Some(data) = request.uri().query() {
                    let url = url::form_urlencoded::parse(data.as_bytes())
                        .filter(|(k, _v)| k.eq("url"))
                        .map(|(_k, v)| v)
                        .next();

                    match url {
                        Some(url) => self.handle_category(url.into_owned()),
                        None => self.handle_missing(),
                    }
                }
                else {
                    self.handle_missing()
                }
            },
            (&Method::GET, "/ready") => self.handle_ready(),
            _ => self.handle_404(),
        };

        handler
    }
}

struct MakeSvc {
    redis: redis::aio::ConnectionManager,
}
impl Service<&AddrStream> for MakeSvc {
    type Response = RtbService;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {

        Poll::Ready(Ok(()))
    }

    fn call(&mut self, socket: &AddrStream) -> Self::Future {
        let remote_addr = socket.remote_addr();
        println!("{:?} Got connection: {}", Utc::now(), remote_addr);

        let redis = self.redis.clone();
        let fut = async move { Ok(RtbService { redis }) };

        Box::pin(fut)
    }
}
