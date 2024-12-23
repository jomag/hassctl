use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub struct StateDto {
    pub attributes: HashMap<String, serde_json::Value>,
    pub entity_id: String,
    pub last_changed: String,
    pub state: String,
}

#[derive(Deserialize, Debug)]
pub struct ServiceFieldDto {
    pub name: Option<String>,
    pub description: Option<String>,
    pub default: Option<Value>,
}

#[derive(Deserialize, Debug)]
pub struct ServiceTargetEntityDto {
    pub domain: Option<Vec<String>>,
    pub supported_features: Option<Vec<u32>>,
}

#[derive(Deserialize, Debug)]
pub struct ServiceTargetDto {
    pub entity: Option<Vec<ServiceTargetEntityDto>>,
}

#[derive(Deserialize, Debug)]
pub struct ServiceDto {
    pub name: String,
    pub description: Option<String>,
    pub fields: HashMap<String, ServiceFieldDto>,
    pub target: Option<ServiceTargetDto>,
}

impl ServiceDto {
    pub fn get_target_entity_domains(&self) -> Option<Vec<String>> {
        self.target
            .as_ref()
            .and_then(|target| target.entity.as_ref())
            .map(|entities| {
                entities
                    .iter()
                    .filter_map(|entity| entity.domain.as_ref())
                    .flat_map(|domains| domains.iter())
                    .cloned()
                    .collect()
            })
    }
}
impl StateDto {
    pub fn friendly_name(&self) -> Option<String> {
        match self.attributes.get("friendly_name") {
            Some(serde_json::Value::String(s)) => Some(s.into()),
            Some(_) => None,
            None => None,
        }
    }

    pub fn name(&self) -> String {
        match self.friendly_name() {
            Some(name) => name,
            None => self.entity_id.clone(),
        }
    }

    pub fn pretty_print(&self, with_attributes: bool) {
        match self.friendly_name() {
            Some(name) => println!("{} - {} ({})", self.entity_id, name, self.state),
            None => println!("{} ({})", self.entity_id, self.state),
        }
        if with_attributes {
            for (key, value) in self.attributes.iter() {
                println!("  - {}: {}", key, value)
            }
        }
    }
}
