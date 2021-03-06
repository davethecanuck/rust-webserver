use std::fs;
use std::fs::metadata;
use std::str;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::io;
use std::sync::Arc;
use regex::bytes::Regex;
use serde::Serialize;
use serde::Deserialize;
use serde_json;

mod app_log_config;
use app_log_config::AppLogConfig;
use log::{info, warn, error};

// Our custom threadpool library
use utils_multiproc::ThreadPool;

// Config options
use std::path::PathBuf;
use structopt::StructOpt;

//-------------------------------------
// Command line arguments
//-------------------------------------
#[derive(Debug, StructOpt)]
#[structopt(name = "rust-webserver", about = "A simple webserver.")]
struct CliOpt {
    /// Set webserver config file
    #[structopt(
        short = "c", 
        long = "config", 
        default_value = "sample/config/localhost.json"
    )]
    config: PathBuf,

    /// Log file directory
    #[structopt(short="l", long="log")]
    log_path: Option<PathBuf>,

    /// One of info/warn/error/debug/trace
    #[structopt(short="v", long="verbosity", default_value="info")]
    verbosity: String,
}

//-------------------------------------
// Server configuration
//-------------------------------------
#[derive(Serialize, Deserialize, Debug)]
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

//-----------------------------------------------
// EYE - Is this okay for handling other error types?
//-----------------------------------------------
type ConnHandlerResult = Result<(), io::Error>;

//-------------------------------------
// Main entry point
//-------------------------------------
fn main() {
    // Parse command line args and setup logging
    let args = CliOpt::from_args();
    let log_config = AppLogConfig::new(args.verbosity, args.log_path);
    log_config.init_flexi_logger();

    info!("Loading server config from {:?}", args.config);
    let config = fs::read_to_string(&args.config).unwrap();
    let server_config: ServerConfig = serde_json::from_str(&config).unwrap();
    start_server(server_config);
}

// Main loop for the server
fn start_server(server_config: ServerConfig) {
    let address = server_config.address();
    let listener = TcpListener::bind(address).unwrap();
    info!("Serving on: {}", server_config.address());
    let pool = ThreadPool::new(4);
    let server_config = Arc::new(server_config);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let thread_config = server_config.clone();
                pool.execute(move || {
                    match handle_stream(stream, thread_config) {
                        Err(e) => error!("Failed to handle_stream: {:?}", e),
                        _ => (),
                    }
                });
            },
            Err(e) => {
                error!("Stream return error: {:?}", e);
            }
        }
    }
}

fn handle_stream(stream: TcpStream, 
        server_config: Arc<ServerConfig>) -> ConnHandlerResult {
    let mut conn_handler = ConnHandler::new(stream, server_config);
    conn_handler.process()?;
    Ok(())
}

//------------------------------------------------
// Implements a single request in a single thread
//------------------------------------------------
struct ConnHandler {
    stream: TcpStream,
    buffer: [u8; 4096],
    header_regex: Regex,
    server_config: Arc<ServerConfig>,
}

impl ConnHandler {
    fn new(stream: TcpStream, server_config: Arc<ServerConfig>) -> ConnHandler {
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
                info!("Request is for {:?}", req);
                self.send_response(&req)?;
            },
            None => {
                error!("Invalid request: \n{}", 
                    String::from_utf8_lossy(&self.buffer));
            }
        }
        Ok(())
    }

    fn get_mime_type(&self, filename: &str) -> String {
        // EYE - Could switch to mime/mime_guess crate but
        // save for if needed
        match filename {
            x if x.ends_with(".html") => String::from("text/html"),
            x if x.ends_with(".css") => String::from("text/css"),
            x if x.ends_with(".jpeg") => String::from("image/jpeg"),
            x if x.ends_with(".jpg") => String::from("image/jpeg"),
            x if x.ends_with(".png") => String::from("image/png"),
            x if x.ends_with(".gif") => String::from("image/gif"),
            x if x.ends_with(".json") => String::from("json"),
            x if x.ends_with(".js") => String::from("application/javascript"),
            _ => String::from("text/html"),
        }
    }

    fn get_response(&self, req: &String) -> (String, Vec<u8>, String) {
        let filename = self.get_filename(req);
        let mime_type = self.get_mime_type(&filename);
        info!("get_response: Reading file={} mime_type={}", 
            filename, mime_type);
        let (status_line, contents) = match fs::read(filename) {
            Ok(c) => (String::from("200 OK"), c),
            Err(e) => (String::from("404 NOT FOUND"), 
                       ConnHandler::get_error_content(&e)),
        };
        (status_line, contents, mime_type)
    }

    fn get_filename(&self, req: &String) -> String {
        let path = format!("{}{}", self.server_config.document_root, req);

        match metadata(path.as_str()) {
            Ok(md) => {
                if md.is_dir() {
                    // Return index.html in that directory
                    return format!("{}/index.html", path);
                }
                else {
                    // It's a file so return directly
                    return path;
                }
            },
            Err(_e) => {
                // Could not find file
                warn!("Could not find file for request: {}", req);
                return path;
            }
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
        info!("Sending mime_type={} content-length={}", 
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
