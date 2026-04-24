use std::collections::HashMap;
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
    pub production_speed: u32,
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
