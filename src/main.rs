use std::convert::Infallible;
use std::net::SocketAddr;
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};

// https://hyper.rs/guides/server/hello-world/

async fn request_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let mut response = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/category") => {
            match req.uri().query() {
                Some(_data) => {
                    *response.body_mut() = Body::from("foo\n");
                    //println!("Data: {}", data);
                },
                None => ()
            };
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

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // TODO: Parameterize listening port
    let address = SocketAddr::from(([0, 0, 0, 0], 8080));

    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(request_handler))
    });

    let server = Server::bind(&address)
        .http1_only(true)
        .serve(make_svc);
    println!("Listening on {}", server.local_addr());

    let graceful = server.with_graceful_shutdown(shutdown_signal());

    if let Err(e) = graceful.await {
        eprintln!("server error: {}", e);
    }

    return Ok(());
}
