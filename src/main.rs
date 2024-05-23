use std::{collections::HashMap, env};

use clap::{Parser, Subcommand};
use reqwest::Error;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

const ACCESS_TOKEN_KEY: &str = "HASSCTL_ACCESS_TOKEN";
const PORT_KEY: &str = "HASSCTL_PORT";
const HOST_KEY: &str = "HASSCTL_HOST";

fn request<T: DeserializeOwned>(path: &str) -> Result<T, Error> {
    let access_token = env::var(ACCESS_TOKEN_KEY).unwrap();
    let host = env::var(HOST_KEY).unwrap();
    let port = env::var(PORT_KEY).unwrap();
    let url = format!("http://{}:{}{}", host, port, path);
    let client = reqwest::blocking::Client::new();
    let response = client.get(url).bearer_auth(access_token).send();
    match response {
        Ok(res) => {
            // println!("TEXT: {:?}", res.text());
            res.json::<T>()
        }
        Err(err) => Err(err),
    }
}

fn post_request<T: Serialize, R: DeserializeOwned>(path: &str, payload: &T) -> Result<R, Error> {
    let access_token = env::var(ACCESS_TOKEN_KEY).unwrap();
    let host = env::var(HOST_KEY).unwrap();
    let port = env::var(PORT_KEY).unwrap();
    let url = format!("http://{}:{}{}", host, port, path);
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url)
        .bearer_auth(access_token)
        .json(payload)
        .send();
    match response {
        Ok(res) => res.json::<R>(),
        Err(err) => Err(err),
    }
}

#[derive(Deserialize, Debug)]
struct StateDto {
    attributes: HashMap<String, serde_json::Value>,
    entity_id: String,
    last_changed: String,
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

fn cmd_scene_list() {
    match request::<Vec<StateDto>>("/api/states") {
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

fn cmd_scene_enable(entity_id: String) {
    let payload = ServiceDataDto { entity_id };

    match post_request::<ServiceDataDto, Vec<StateDto>>("/api/services/scene/turn_on", &payload) {
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

    match &cli.command {
        Commands::Scene(scene_cli) => match &scene_cli.command {
            SceneCommands::List => cmd_scene_list(),
            SceneCommands::Show => todo!(),
            SceneCommands::Enable { entity_id } => cmd_scene_enable(entity_id.clone()),
        },
    }
}
