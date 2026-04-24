use std::{
    path::Path,
    collections::HashMap,
};
use serde::Deserialize;

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
    pub production_rules: Vec<String>,
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
    pub buildings: HashMap<String, Building>,
    pub categories: HashMap<String, Category>,
    pub recipes: HashMap<String, Recipe>,
}

impl Assets {
    pub fn load(dir: &Path) -> Result<Self, AssetError> {
        Ok(Self {
            items: load_json(&dir.join("items.json"))?,
            buildings: load_json(&dir.join("buildings.json"))?,
            categories: load_json(&dir.join("categories.json"))?,
            recipes: load_json(&dir.join("recipes.json"))?,
        })
    }
}

fn load_json<T>(dir: &Path) -> Result<HashMap<String, T>, AssetError> 
where T: for<'de> Deserialize<'de> + Identifiable {
    let asset = std::fs::read_to_string(dir)
        .map_err(AssetError::IoError)?;
    
    let entries: Vec<T> = serde_json::from_str(&asset)
        .map_err(AssetError::ParseError)?;

    let hashmap = entries.into_iter().map(|e| (e.id().to_owned(), e)).collect();
    
    Ok(hashmap)
}

pub trait Identifiable {
    fn id(&self) -> &str;
}

macro_rules! has_id {
    ($($t:ty), *) => {
        $(impl Identifiable for $t { fn id(&self) -> &str { &self.id } })*
    };
}
has_id!(Item, Building, Category, Recipe);

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
