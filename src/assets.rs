use serde::Deserialize;
use std::{collections::HashMap, path::Path};

pub type ItemList = Vec<ItemStack>;

#[derive(Debug, Clone, Deserialize)]
pub struct ItemStack {
    pub id: String,
    pub count: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub stack_limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Worker {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub stack_limit: usize,
    pub speed: f32,
    pub inventory_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Building {
    pub id: String,
    pub name: String,
    pub description: String,
    pub base_cost: ItemList,
    pub cost_increase: f32,
    pub first_free: bool,
    pub tier_up_from: Option<String>,
    pub x_size: usize,
    pub y_size: usize,
    pub inventory_size: usize,
    pub recipes: Vec<String>,
    pub production_speed: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub inputs: ItemList,
    pub outputs: ItemList,
    pub time: u32,
}

#[derive(Debug)]
pub struct Assets {
    pub items: HashMap<String, Item>,
    pub workers: HashMap<String, Worker>,
    pub buildings: HashMap<String, Building>,
    pub categories: HashMap<String, Category>,
    pub recipes: HashMap<String, Recipe>,
}

/// All static game data loaded and initialized once at startup.
/// Each collection is keyed by the id of the asset.
impl Assets {
    /// Loads all asset files.
    ///
    /// # Errors
    /// Returns an error if any file fails to load or parse.
    pub fn load(dir: &Path) -> Result<Self, AssetError> {
        Ok(Self {
            items: load_json(&dir.join("items.json"))?,
            workers: load_json(&dir.join("workers.json"))?,
            buildings: load_json(&dir.join("buildings.json"))?,
            categories: load_json(&dir.join("categories.json"))?,
            recipes: load_json(&dir.join("recipes.json"))?,
        })
    }
}

/// Loads a JSON file containing an array of T and deserializes it.
fn load_json<T>(dir: &Path) -> Result<HashMap<String, T>, AssetError>
where
    T: for<'de> Deserialize<'de> + Identifiable,
{
    let asset = std::fs::read_to_string(dir).map_err(AssetError::IoError)?;
    let entries: Vec<T> = serde_json::from_str(&asset).map_err(AssetError::ParseError)?;
    let hashmap = entries
        .into_iter()
        .map(|e| (e.id().to_owned(), e))
        .collect();

    Ok(hashmap)
}

/// Implemented by all asset types for keying them by ID generically.
pub trait Identifiable {
    fn id(&self) -> &str;
}

macro_rules! has_id {
    ($($t:ty), *) => {
        $(impl Identifiable for $t { fn id(&self) -> &str { &self.id } })*
    };
}
has_id!(Item, Worker, Building, Category, Recipe);

#[derive(Debug)]
pub enum AssetError {
    IoError(std::io::Error),
    ParseError(serde_json::Error),
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AssetError::IoError(err) => write!(f, "IO Error: {}", err),
            AssetError::ParseError(err) => write!(f, "Parse Error: {}", err),
        }
    }
}

impl std::error::Error for AssetError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn deserialize<T>(json: &str) -> HashMap<String, T>
    where
        T: for<'de> Deserialize<'de> + Identifiable,
    {
        let entries: Vec<T> = serde_json::from_str(json).expect("Failed to parse test JSON");
        entries
            .into_iter()
            .map(|e| (e.id().to_owned(), e))
            .collect()
    }

    #[test]
    fn test_category() {
        let json = r#"[{
            "id": "worker",
            "name": "Worker",
            "description": "Carries items."
        }]"#;
        let hashmap: HashMap<String, Category> = deserialize(json);
        let category = hashmap.get("worker").expect("Worker category missing");
        assert_eq!(category.name, "Worker");
    }

    #[test]
    fn test_item() {
        let json = r#"[{
            "id": "iron_ore",
            "name": "Iron Ore",
            "category": "ore",
            "description": "Raw and impure iron.",
            "stack_limit": 256
        }]"#;
        let hashmap: HashMap<String, Item> = deserialize(json);
        let item = hashmap.get("iron_ore").expect("Iron Ore missing");
        assert_eq!(item.stack_limit, 256);
    }

    #[test]
    fn test_faulty_json() {
        let json = r#"[{
            "id": "bad",
            "name": "Bad"
        }]"#;
        let result: Result<Vec<Item>, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
