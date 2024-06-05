use std::{collections::HashMap, env};

use clap::{Parser, Subcommand};
use reqwest::Error;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

const NAME: &str = "hassctl";
const ACCESS_TOKEN_KEY: &str = "HASSCTL_ACCESS_TOKEN";
const PORT_KEY: &str = "HASSCTL_PORT";
const DEFAULT_PORT: u16 = 8123;
const HOST_KEY: &str = "HASSCTL_HOST";

struct Client {
    access_token: String,
    port: u16,
    host: String,
}

enum ClientError {
    MissingAccessToken,
    InvalidAccessToken,
    MissingHost,
    InvalidHost,
    InvalidPort,
}

impl ClientError {
    fn error_description(&self) -> String {
        match self {
            ClientError::MissingAccessToken => format!(
                "Missing access token\n\
                \n\
                To authenticate requests to Home Assistant, {}\n\
                needs to have an access token.\n\
                \n\
                Go to your profile in the Home Assistant dashboard,\n\
                and select the Security tab. Create a long-lived token,\n\
                and make sure to copy the token.\n\
                \n\
                Then create an environment variable named {}\n\
                with the access token as value.",
                NAME, ACCESS_TOKEN_KEY
            ),
            ClientError::InvalidAccessToken => "Invalid access token!".into(),
            ClientError::MissingHost => "Missing host.".into(),
            ClientError::InvalidHost => "Invalid host.".into(),
            ClientError::InvalidPort => "Invalid port.".into(),
        }
    }
}

impl Client {
    fn setup() -> Result<Self, ClientError> {
        let access_token = match env::var(ACCESS_TOKEN_KEY) {
            Ok(s) => s,
            Err(env::VarError::NotPresent) => return Err(ClientError::MissingAccessToken),
            Err(_) => return Err(ClientError::InvalidAccessToken),
        };

        let host = match env::var(HOST_KEY) {
            Ok(s) => s,
            Err(env::VarError::NotPresent) => return Err(ClientError::MissingHost),
            Err(_) => return Err(ClientError::InvalidHost),
        };

        let port = match env::var(PORT_KEY) {
            Ok(s) => match s.parse::<u16>() {
                Ok(v) => v,
                Err(_) => return Err(ClientError::InvalidPort),
            },
            Err(env::VarError::NotPresent) => DEFAULT_PORT,
            Err(_) => return Err(ClientError::InvalidPort),
        };

        Ok(Self {
            access_token,
            host,
            port,
        })
    }

    fn build_url(&self, path: &str) -> String {
        format!("http://{}:{}{}", self.host, self.port, path)
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        let url = self.build_url(path);
        let client = reqwest::blocking::Client::new();
        let response = client.get(url).bearer_auth(&self.access_token).send();
        match response {
            Ok(res) => res.json::<T>(),
            Err(err) => Err(err),
        }
    }

    fn post<T: Serialize, R: DeserializeOwned>(&self, path: &str, payload: &T) -> Result<R, Error> {
        let url = self.build_url(path);
        let client = reqwest::blocking::Client::new();
        let response = client
            .post(url)
            .bearer_auth(&self.access_token)
            .json(payload)
            .send();
        match response {
            Ok(res) => res.json::<R>(),
            Err(err) => Err(err),
        }
    }
}

#[derive(Deserialize, Debug)]
struct StateDto {
    attributes: HashMap<String, serde_json::Value>,
    entity_id: String,
    // last_changed: String,
    state: String,
}

impl StateDto {
    fn friendly_name(&self) -> Option<String> {
        match self.attributes.get("friendly_name") {
            Some(serde_json::Value::String(s)) => Some(s.into()),
            Some(_) => None,
            None => None,
        }
    }

    fn name(&self) -> String {
        match self.friendly_name() {
            Some(name) => name,
            None => self.entity_id.clone(),
        }
    }
}

fn cmd_scene_list(client: &Client) {
    match client.get::<Vec<StateDto>>("/api/states") {
        Ok(list) => {
            let scenes: Vec<StateDto> = list
                .into_iter()
                .filter(|s| s.entity_id.starts_with("scene."))
                .collect();

            println!("{} scenes found:", scenes.len());
            for state in scenes {
                match state.friendly_name() {
                    Some(name) => println!(
                        "Scene: {:?} ({:?}). State: {:?}",
                        name, state.entity_id, state.state
                    ),
                    None => println!("Scene: {:?}. State: {:?}", state.entity_id, state.state),
                }
                println!(" - Attributes:");
                for (key, val) in state.attributes {
                    println!("   {}: {:?}", key, val);
                }
                println!("")
            }
        }
        Err(err) => println!("Failed to fetch scene list: {:?}", err),
    }
}

#[derive(Serialize)]
struct ServiceDataDto {
    entity_id: String,
}

fn cmd_scene_enable(client: &Client, entity_id: String) {
    let payload = ServiceDataDto { entity_id };

    match client.post::<ServiceDataDto, Vec<StateDto>>("/api/services/scene/turn_on", &payload) {
        Ok(_) => println!("Scene enabled."),
        Err(_) => println!("Failed to enable scene."),
    }
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Scene(SceneCli),
}

#[derive(Parser)]
struct SceneCli {
    #[command(subcommand)]
    command: SceneCommands,
}

#[derive(Subcommand)]
enum SceneCommands {
    List,
    Show,
    Enable { entity_id: String },
}

fn main() {
    let cli = Cli::parse();

    let client = match Client::setup() {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to create client:\n\n{}", e.error_description());
            return;
        }
    };

    match &cli.command {
        Commands::Scene(scene_cli) => match &scene_cli.command {
            SceneCommands::List => cmd_scene_list(&client),
            SceneCommands::Show => todo!(),
            SceneCommands::Enable { entity_id } => cmd_scene_enable(&client, entity_id.clone()),
        },
    }
}
