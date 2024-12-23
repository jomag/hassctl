mod dto;

use std::{collections::HashMap, env};

use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use dto::{ServiceDto, StateDto};
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
            ClientError::MissingHost => format!(
                "Missing host.\n\
                \n\
                Host must be specified in the environment variable {}.\n",
                HOST_KEY
            )
            .into(),
            ClientError::InvalidHost => "Invalid host.".into(),
            ClientError::InvalidPort => "Invalid port.".into(),
        }
    }
}

impl Client {
    fn setup() -> Result<Self, ClientError> {
        let access_token = match env::var(ACCESS_TOKEN_KEY) {
            Ok(s) => s,
            Err(env::VarError::NotPresent) => match dotenv::var(ACCESS_TOKEN_KEY) {
                Ok(s) => s,
                Err(_) => return Err(ClientError::MissingAccessToken),
            },
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

    fn fetch_services(&self) -> Result<Vec<ServiceDomainDto>, Error> {
        self.get::<Vec<ServiceDomainDto>>("/api/services")
    }

    fn fetch_entities(&self) -> Result<Vec<StateDto>, Error> {
        self.get::<Vec<StateDto>>("/api/states")
    }

    fn fetch_entities_by_domain(&self, domains: Vec<String>) -> Result<Vec<StateDto>, Error> {
        let all = self.fetch_entities()?;
        Ok(all
            .into_iter()
            .filter(|e| {
                domains
                    .iter()
                    .any(|d| e.entity_id.starts_with(d) && e.entity_id[d.len()..].starts_with('.'))
            })
            .collect())
    }

    // FIXME: not all services require entity ID's
    fn call_service(
        &self,
        domain: &str,
        service: &str,
        entity_id: &str,
    ) -> Result<Vec<StateDto>, Error> {
        let payload = ServiceDataDto {
            entity_id: entity_id.to_string(),
        };

        self.post::<ServiceDataDto, Vec<StateDto>>(
            format!("/api/services/{}/{}", domain, service).as_str(),
            &payload,
        )
    }
}

#[derive(Deserialize, Debug)]
struct ServiceDomainDto {
    domain: String,
    services: HashMap<String, ServiceDto>,
}

fn cmd_entity_list(client: &Client) {
    match client.get::<Vec<StateDto>>("/api/states") {
        Ok(list) => {
            for state in list {
                state.pretty_print(false);
            }
        }
        Err(err) => println!("Failed to fetch entity list: {:?}", err),
    }
}

fn cmd_entity_show(client: &Client, entity_id: &str) {
    match client.get::<StateDto>(format!("/api/states/{}", entity_id).as_str()) {
        Ok(state) => state.pretty_print(true),
        Err(err) => println!("Failed to fetch entity list: {:?}", err),
    }
}

fn cmd_service_list(client: &Client) {
    match client.get::<Vec<ServiceDomainDto>>("/api/services") {
        Ok(list) => {
            for domain in list {
                println!("\nDomain: {}", domain.domain);
                for (svc_id, svc) in domain.services.iter() {
                    match &svc.description {
                        Some(descr) => println!("  - '{}' - {}: {}", svc_id, svc.name, descr),
                        None => println!("  - '{}' - {}", svc_id, svc.name),
                    }
                }
            }
        }
        Err(err) => println!("Failed to fetch service list: {:?}", err),
    }
}

fn prompt_for_selection(prompt: &str, options: &Vec<&str>) -> usize {
    FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&options)
        .max_length(16)
        .interact()
        .unwrap()
}

fn cmd_call(client: &Client) -> Result<(), Error> {
    let domains = client.fetch_services()?;
    let domain_ids = domains.iter().map(|d| d.domain.as_str()).collect();
    let i = prompt_for_selection("Domain", &domain_ids);
    let domain_id = domain_ids[i];
    let domain = domains.iter().find(|d| d.domain == domain_id).unwrap();

    let service_ids = domain.services.keys().map(|k| k.as_str()).collect();
    let i = prompt_for_selection("Service", &service_ids);
    let service_id = service_ids[i];
    let service = domain.services.get(service_id).unwrap();

    let entities = match service.get_target_entity_domains() {
        None => client.fetch_entities()?,
        Some(d) => client.fetch_entities_by_domain(d)?,
    };
    let entity_ids = entities.iter().map(|e| e.entity_id.as_str()).collect();
    let i = prompt_for_selection("Entity", &entity_ids);
    let entity_id = entity_ids[i];

    let _ = client.call_service(domain_id, service_id, entity_id)?;

    println!("Service called successfully");
    Ok(())
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
    Entity(EntityCli),
    Service(ServiceCli),
    Call(CallCli),
}

#[derive(Parser)]
struct CallCli {}

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

#[derive(Parser)]
struct EntityCli {
    #[command(subcommand)]
    command: EntityCommands,
}

#[derive(Subcommand)]
enum EntityCommands {
    List,
    Show { entity_id: String },
}

#[derive(Parser)]
struct ServiceCli {
    #[command(subcommand)]
    command: ServiceCommands,
}

#[derive(Subcommand)]
enum ServiceCommands {
    List,
}

fn main() {
    dotenv::dotenv().ok();
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
        Commands::Entity(entity_cli) => match &entity_cli.command {
            EntityCommands::List => cmd_entity_list(&client),
            EntityCommands::Show { entity_id } => cmd_entity_show(&client, entity_id),
        },
        Commands::Service(service_cli) => match &service_cli.command {
            ServiceCommands::List => cmd_service_list(&client),
        },
        Commands::Call(_) => {
            let _ = cmd_call(&client);
        }
    }
}
