use std::task::Context;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;
use chrono::prelude::Utc;
use core_affinity;
use futures::task::Poll;
use futures::Future;
use futures::future;
use futures::future::FutureExt;
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
    // TODO: Use async multiplexed connection and connection manager
    // https://docs.rs/redis/0.16.0/redis/struct.Client.html
    // https://doc.rust-lang.org/book/ch16-03-shared-state.html
    // https://doc.rust-lang.org/std/thread/struct.LocalKey.html
    // https://stackoverflow.com/questions/53038935/cannot-move-out-of-captured-variables-in-an-fnmut-closure

    //let client = redis::cluster::ClusterClient::open(nodes).unwrap();
    redis::Client::open(connection_info).unwrap().get_tokio_connection_manager().await.unwrap()
}

fn get_segments(key: String) -> String {
    // println!("{}", key);

    String::from("\n")
}

async fn request_handler(request: Request<Body>) -> Result<Response<Body>, Infallible> {
    let mut response = Response::new(Body::empty());

    match (request.method(), request.uri().path()) {
        (&Method::GET, "/category") => {
            if let Some(data) = request.uri().query() {
                let url = url::form_urlencoded::parse(data.as_bytes())
                    .filter(|(k, _v)| k.eq("url"))
                    .map(|(_k, v)| v)
                    .next();


                let segments = match url {
                    Some(url) => get_segments(url.into()),
                    None => "".to_string(),
                };

                *response.body_mut() = Body::from(segments);
                //println!("Data: {}", data);
            }
        },

        (&Method::GET, "/ready") => {
            *response.body_mut() = Body::from("1\n");
        },

        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        },
    };

    Ok(response)
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

    // Trying to share this with the asynchronous request handlers to be able to
    // reuse the connection to Redis.
    // ConnectionManager implements Send and Sync
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


struct RtbService {
    redis: redis::aio::ConnectionManager,
}
impl Service<Request<Body>> for RtbService {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {

        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {

        let mut redis = self.redis.clone();

        let fut = async move {
            let result: Result<String, RedisError> = redis.get("foo").await;

            let segments: String = match result {
                Ok(data) => data,
                _ => "".to_string(),
            };

            Ok(Response::new(Body::from(segments)))
        };

        Box::pin(fut)
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
