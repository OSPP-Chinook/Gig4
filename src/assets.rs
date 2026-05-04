use serde::Deserialize;
use std::{collections::HashMap, path::Path};

pub struct Assets {
    pub items: HashMap<String, ItemAsset>,
    pub workers: HashMap<String, WorkerAsset>,
    pub buildings: HashMap<String, BuildingAsset>,
    pub recipes: HashMap<String, RecipeAsset>,
    pub categories: HashMap<String, CategoryAsset>,
}

/// All static game data is loaded and initialized once at startup.
impl Assets {
    /// Loads all assets from the given directory.
    ///
    /// # Errors
    ///
    /// Returns `AssetError::IoError` if the directory cannot be read.
    /// Returns `AssetError::ParseError` if any of the JSON files cannot be parsed.
    pub fn load(dir: &Path) -> Result<Self, AssetError> {
        Ok(Self {
            items: load_asset(&dir.join("items.json"))?,
            workers: load_asset(&dir.join("workers.json"))?,
            buildings: load_asset(&dir.join("buildings.json"))?,
            recipes: load_asset(&dir.join("recipes.json"))?,
            categories: load_asset(&dir.join("categories.json"))?,
        })
    }
}

/// Reads a file and returns its content as a string.
///
/// # Errors
///
/// Returns `AssetError::IoError` if the file cannot be read.
fn read_json(path: &Path) -> Result<String, AssetError> {
    std::fs::read_to_string(path).map_err(AssetError::IoError)
}

/// Deserializes a JSON string into a hash map of any asset type accepted by Identifiable, keyed by its id.
///
/// # Errors
///
/// Returns `AssetError::ParseError` if the JSON string cannot be parsed.
fn parse<T>(json: &str) -> Result<HashMap<String, T>, AssetError>
where
    T: for<'de> Deserialize<'de> + Identifiable,
{
    let entries: Vec<T> = serde_json::from_str(json).map_err(AssetError::ParseError)?;
    Ok(entries
        .into_iter()
        .map(|e| (e.id().to_owned(), e))
        .collect())
}

fn load_asset<T>(path: &Path) -> Result<HashMap<String, T>, AssetError>
where
    T: for<'de> Deserialize<'de> + Identifiable,
{
    parse(&read_json(path)?)
}

/// Trait for assets that can be identified by a string ID.
trait Identifiable {
    fn id(&self) -> &str;
}

macro_rules! impl_identifiable {
    ($($t:ty), *) => {
        $(impl Identifiable for $t { fn id(&self) -> &str { &self.id } })*
    };
}

impl_identifiable!(
    ItemAsset,
    WorkerAsset,
    BuildingAsset,
    CategoryAsset,
    RecipeAsset
);

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

pub type ItemList = Vec<ItemStack>;

#[derive(Debug, Clone, Deserialize)]
pub struct ItemStack {
    pub id: String,
    pub count: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemAsset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub stack_limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkerAsset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub stack_limit: usize,
    pub inventory_size: usize,
    pub speed: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BuildingAsset {
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
pub struct RecipeAsset {
    pub id: String,
    pub inputs: ItemList,
    pub outputs: ItemList,
    pub time: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CategoryAsset {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item() {
        let json = r#"[{
            "id": "iron_ore",
            "name": "Iron Ore",
            "category": "ore",
            "description": "Raw and impure iron.",
            "stack_limit": 256
        }]"#;
        let map: HashMap<String, ItemAsset> = parse(json).unwrap();
        let item = map.get("iron_ore").expect("Iron Ore missing");
        assert_eq!(item.name, "Iron Ore");
        assert_eq!(item.category, "ore");
        assert_eq!(item.stack_limit, 256);
    }

    #[test]
    fn test_category() {
        let json = r#"[{
            "id": "worker",
            "name": "Worker",
            "description": "Carries items."
        }]"#;
        let map: HashMap<String, CategoryAsset> = parse(json).unwrap();
        let category = map.get("worker").expect("Worker missing");
        assert_eq!(category.name, "Worker");
        assert_eq!(category.description, "Carries items.");
    }

    #[test]
    fn test_faulty_json() {
        let json = r#"[{
            "id": "bad",
            "name": "Bad"
        }]"#;
        let result: Result<HashMap<String, ItemAsset>, AssetError> = parse(json);
        assert!(result.is_err());
    }
}
