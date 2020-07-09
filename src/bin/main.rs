use hello::ThreadPool;
use std::fs;
use std::str;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::time::Duration;
use std::thread;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let pool = ThreadPool::new(4);

    // EYE - test to shut down on 2nd connection
    //for stream in listener.incoming().take(2) {
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        pool.execute(|| {
            handle_connection(stream);
        });
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 512];
    match stream.read(&mut buffer) {
        Ok(_) => {
            // Uncomment to print request
            println!("Read buffer: \n{}", String::from_utf8_lossy(&buffer));

            let get = b"GET / HTTP/1.1\r\n";
            let sleep = b"GET /sleep HTTP/1.1\r\n";
            thread::sleep(Duration::from_secs(1));
    
            let (status_line, file) = if buffer.starts_with(get) {
                ("200 OK", "htdocs/hello.html")
            }
            else if buffer.starts_with(sleep) {
                thread::sleep(Duration::from_secs(8));
                ("200 OK", "htdocs/sleep.html")
            } 
            else {
                ("404 NOT FOUND", "htdocs/404.html")
            };
            send_response(&stream, &status_line, &file);
        },
        Err(e) => println!("Stream read failed with: {}", e),
    }
}

fn send_response(mut stream: &TcpStream, status_line: &str, file: &str) {
    match fs::read_to_string(file) {
        Err(e) => println!("Failed to read file {}: {}", file, e),
        Ok(contents) => {
            let response = to_http(status_line, contents);
            println!("Sending response: \n{}", 
                     String::from_utf8_lossy(response.as_bytes()));
            stream.write(response.as_bytes()).unwrap();
            stream.flush().unwrap();
        }
    }
}

fn to_http(status_line: &str, contents: String) -> String {
    format!("
HTTP/1.1 {}
Content-Length: {}

{}
    ", status_line, contents.len(), contents)
}
