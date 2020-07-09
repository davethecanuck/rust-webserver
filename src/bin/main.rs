use hello::ThreadPool;
use std::fs;
use std::str;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::time::Duration;
use std::thread;
use std::io;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let pool = ThreadPool::new(4);

    // NOTE - test to shut down on 2nd connection
    //for stream in listener.incoming().take(2) {
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        pool.execute(|| {
            match handle_connection(stream) {
                Err(e) => println!("Failed to handle_connection: {:?}", e),
                _ => (),
            }
        });
    }
}

type ConnHandlerResult = Result<(), io::Error>;

fn handle_connection(stream: TcpStream) -> ConnHandlerResult {
    let mut conn_handler = ConnHandler::new(stream);
    conn_handler.process()?;
    Ok(())
}

struct ConnHandler {
    stream: TcpStream,
    buffer: [u8; 1024],
}

impl ConnHandler {
    // Possibly do all error handling here so we can
    // send appropriate response to client for some errors
    fn process(&mut self) -> ConnHandlerResult {
        self.stream.read(&mut self.buffer)?;
        println!("Read buffer: \n{}", String::from_utf8_lossy(&self.buffer));

        let get = b"GET / HTTP/1.1\r\n";
        let sleep = b"GET /sleep HTTP/1.1\r\n";
        thread::sleep(Duration::from_secs(1));
    
        let (status_line, file) = if self.buffer.starts_with(get) {
            ("200 OK", "htdocs/hello.html")
        }
        else if self.buffer.starts_with(sleep) {
            thread::sleep(Duration::from_secs(8));
            ("200 OK", "htdocs/sleep.html")
        } 
        else {
            ("404 NOT FOUND", "htdocs/404.html")
        };
        self.send_response(&status_line, &file)?;
        Ok(())
    }

    fn new(stream: TcpStream) -> ConnHandler {
        ConnHandler{ 
            stream,
            buffer: [0_u8; 1024],
        }
    }

    fn send_response(&mut self, status_line: &str, file: &str) -> ConnHandlerResult {
        let contents = fs::read_to_string(file)?;
        let response = to_http(status_line, contents);
        println!("Sending response: \n{}", 
             String::from_utf8_lossy(response.as_bytes()));
        self.stream.write(response.as_bytes()).unwrap();
        self.stream.flush().unwrap();
        Ok(())
    }
}

fn to_http(status_line: &str, contents: String) -> String {
    format!("
HTTP/1.1 {}
Content-Length: {}

{}
    ", status_line, contents.len(), contents)
}
