use std::fs;
use std::str;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate clap;
use clap::App;

mod config {
    pub fn args() {
        // Load command line arguments defined in cli.yml
        let cli = load_yaml!("cli.yml");
        App::from_yaml(cli).get_matches()
    }
         
    pub fn config() {
        let config_file = args().value_of("config")
            .unwrap_or("config/localhost.json");
        let config = fs::read_to_string(&config_file).unwrap();
        let server_config: ServerConfig = serde_json::from_str(&config).unwrap();
        server_config
    }
}

// Server configuration
#[derive(Serialize, Deserialize)]
struct ServerConfig {
    host: String,
    port: u32,
    document_root: String,
}

