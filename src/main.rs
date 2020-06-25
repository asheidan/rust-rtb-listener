use std::io::Write;
use std::io::Read;
use std::net::TcpStream;
use std::net::TcpListener;

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080").unwrap();
    let address = listener.local_addr().unwrap();

    println!("Listening on: http://{}/", address);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream);
    }

    return Ok(());
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 512];

    stream.read(&mut buffer).unwrap();

    //println!("Request: {}", String::from_utf8_lossy(&buffer[..]));

    let response = "HTTP/1.1 200 OK\r\nConnection keep-alive\r\n\r\n";

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
