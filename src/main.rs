use std::convert::Infallible;
use std::net::SocketAddr;
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use redis::Commands;

// https://hyper.rs/guides/server/hello-world/

fn get_segments(key: String) -> String {
    //let nodes = vec!["redis://192.168.0.134/"];
    //let client = redis::cluster::ClusterClient::open(nodes).unwrap();
    let client = redis::Client::open("redis://192.168.0.134/").unwrap();

    let mut connection = client.get_connection().unwrap();

    match connection.get(key) {
        Ok(data) => data,
        Err(_) => "".to_string(),
    }
}

async fn request_handler(request: Request<Body>) -> Result<Response<Body>, Infallible> {
    let mut response = Response::new(Body::empty());

    match (request.method(), request.uri().path()) {
        (&Method::GET, "/category") => {
            if let Some(data) = request.uri().query() {
                let url = url::form_urlencoded::parse(data.as_bytes())
                    .filter(|(k, _v)| k.eq("url"))
                    .map(|(_k, v)| v)
                    .next().unwrap();

                let segments = get_segments(url.into());
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
