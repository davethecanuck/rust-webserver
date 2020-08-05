use std::fs;
use std::str;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::io;
use regex::bytes::Regex;
use serde::Serialize;
use serde::Deserialize;
use serde_json;

// Our custom threadpool library
use utils_multiproc::ThreadPool;

// Config options
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "rust-webserver", about = "A simple webserver.")]
struct CliOpt {
    /// Set config file
    #[structopt(
        short = "c", 
        long = "config", 
        default_value = "sample/config/localhost.json"
    )]
    config: PathBuf,
}

// Server configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfig {
    host: String,
    port: u32,
    document_root: String,
}

impl ServerConfig {
    fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

// Main entry point
fn main() {
    // Load command line arguments defined in cli.yml
    let args = CliOpt::from_args();
    println!("Loading server config from {:?}", args.config);
    let config = fs::read_to_string(&args.config).unwrap();
    let server_config: ServerConfig = serde_json::from_str(&config).unwrap();
    let address = server_config.address();
    println!("Serving on: {}", address);

    let listener = TcpListener::bind(address).unwrap();
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        // Cheat and clone config for each stream
        let stream = stream.unwrap();
        let c = server_config.clone();
        pool.execute(|| {
            match handle_stream(stream, c) {
                Err(e) => println!("Failed to handle_stream: {:?}", e),
                _ => (),
            }
        });
    }
}

// EYE - Is this okay for handling other error types?
type ConnHandlerResult = Result<(), io::Error>;

fn handle_stream(stream: TcpStream, server_config: ServerConfig) -> ConnHandlerResult {
    let mut conn_handler = ConnHandler::new(stream, server_config);
    conn_handler.process()?;
    Ok(())
}

// Implements a single request in a single thread
struct ConnHandler {
    stream: TcpStream,
    buffer: [u8; 4096],
    header_regex: Regex,
    server_config: ServerConfig,
}

impl ConnHandler {
    fn new(stream: TcpStream, server_config: ServerConfig) -> ConnHandler {
        ConnHandler{
            stream,
            buffer: [0_u8; 4096],
            header_regex: Regex::new(r"^\s*GET (\S+)").unwrap(),
            server_config: server_config,
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
        match self.get_request() {
            Some(req) => {
                if req == "/die" {
                    panic!("I've been told to die!");
                }
                println!("Request is for {:?}", req);
                self.send_response(&req)?;
            },
            None => {
                println!("Invalid request: \n{}", 
                    String::from_utf8_lossy(&self.buffer));
            }
        }
        Ok(())
    }

    fn get_mime_type(&self, filename: &str) -> String {
        // EYE Rudimentary - maybe use regex 
        match filename {
            x if x.ends_with(".html") => String::from("text/html"),
            x if x.ends_with(".jpeg") => String::from("image/jpeg"),
            x if x.ends_with(".jpg") => String::from("image/jpeg"),
            x if x.ends_with(".png") => String::from("image/png"),
            _ => String::from("text/html"),
        }
    }

    fn get_response(&self, req: &String) -> (String, Vec<u8>, String) {
        let filename = self.get_filename(req);
        let mime_type = self.get_mime_type(&filename);
        println!("get_response: Reading file={} mime_type={}", 
                 filename, mime_type);
        let (status_line, contents) = match fs::read(filename) {
            Ok(c) => (String::from("200 OK"), c),
            Err(e) => (String::from("404 NOT FOUND"), 
                       ConnHandler::get_error_content(&e)),
        };
        (status_line, contents, mime_type)
    }

    fn get_filename(&self, req: &String) -> String {
        // EYE - Make much more flexible
        if req.ends_with(".jpg") || req.ends_with(".png") || req.ends_with(".html") {
            // Fully specified file
            return format!("{}{}", self.server_config.document_root, req);
        }
        else if req.ends_with("/") {
            // Directory - default to index.html
            return format!("{}{}index.html", self.server_config.document_root, req);
        }
        else {
            // Shortcut to file - add .html
            return format!("{}{}.html", self.server_config.document_root, req);
        }
    }

    fn send_response(&mut self, req: &String) -> ConnHandlerResult {
        let (status_line, contents, mime_type) = self.get_response(&req);
        let response = ConnHandler::to_http(&status_line, &contents, &mime_type);
        self.stream.write(&response)?;
        self.stream.flush()?;
        Ok(())
    }

    fn get_error_content(e: &std::io::Error) -> Vec<u8> {
       format!("<html><a>Failed to load page: {}</a></html>", e)
           .as_bytes().to_vec()
    }

    fn to_http(status_line: &str, contents: &Vec<u8>, mime_type: &str) -> Vec<u8> {
        println!("Sending mime_type={} content-length={}", 
                 mime_type, contents.len());
        let mut response = format!("
HTTP/1.1 {}
Content-Type: {}
Content-Length: {}\n\n", 
            status_line, mime_type, contents.len())
            .as_bytes()
            .to_vec();
        response.extend(contents);
        response
    }
}
