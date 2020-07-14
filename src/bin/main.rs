use hello::ThreadPool;
use std::fs;
use std::str;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::io;
use regex::bytes::Regex;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let pool = ThreadPool::new(4);

    // NOTE - test to shut down on 2nd connection
    //for stream in listener.incoming().take(2) {
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        pool.execute(|| {
            match handle_stream(stream) {
                Err(e) => println!("Failed to handle_stream: {:?}", e),
                _ => (),
            }
        });
    }
}

type ConnHandlerResult = Result<(), io::Error>;

fn handle_stream(stream: TcpStream) -> ConnHandlerResult {
    let mut conn_handler = ConnHandler::new(stream);
    conn_handler.process()?;
    Ok(())
}

struct ConnHandler {
    stream: TcpStream,
    buffer: [u8; 4096],
    header_regex: Regex,
}

impl ConnHandler {
    fn new(stream: TcpStream) -> ConnHandler {
        ConnHandler{
            stream,
            buffer: [0_u8; 4096],
            header_regex: Regex::new(r"GET (\S+?)\s+").unwrap(),
        }
    }

    fn get_request(&self) -> Option<String> {
        match self.header_regex.captures(&self.buffer) {
            Some(cap) => {
                match String::from_utf8(cap[1].to_vec()) {
                    Ok(s) => Some(s),
                    _ => None,
                }
            }
            _ => None
        }
    }

    // Possibly do all error handling here so we can
    // send appropriate response to client for some errors
    fn process(&mut self) -> ConnHandlerResult {
        self.stream.read(&mut self.buffer)?;
        //println!("Read buffer: \n{}", String::from_utf8_lossy(&self.buffer));
        match self.get_request() {
            Some(req) => {
                println!("Request is for {:?}", req);
                let (status_line, file) = match req.as_str() {
                    "/" => ("200 OK", "htdocs/hello.html"),
                    "/sleep" => ("200 OK", "htdocs/sleep.html"),
                    _ => ("404 NOT FOUND", "htdocs/404.html")
                };
                self.send_response(&status_line, &file)?;
            },
            None => {
                println!("Invalid request: \n{}", 
                    String::from_utf8_lossy(&self.buffer));
            }
        }
        Ok(())
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
