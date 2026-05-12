use serde::Deserialize;
use std::{collections::HashMap, hash::Hash, path::Path, sync::Arc};

pub struct Assets {
    pub items: HashMap<ItemId, ItemAsset>,
    pub workers: HashMap<WorkerId, WorkerAsset>,
    pub buildings: HashMap<BuildingId, BuildingAsset>,
    pub recipes: HashMap<RecipeId, RecipeAsset>,
    pub categories: HashMap<CategoryId, CategoryAsset>,
}

/// All static game data is loaded and initialized once at startup.
impl Assets {
    /// Loads all assets from the given directory.
    ///
    /// # Errors
    ///
    /// Returns `AssetError::Io` if the directory cannot be read.
    /// Returns `AssetError::Parse` if any of the JSON files cannot be parsed.
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
/// Returns `AssetError::Io` if the file cannot be read.
fn read_json(path: &Path) -> Result<String, AssetError> {
    std::fs::read_to_string(path).map_err(AssetError::Io)
}

/// Deserializes a JSON string into a hash map of any asset type accepted by Identifiable.
///
/// # Errors
///
/// Returns `AssetError::Parse` if the JSON string cannot be parsed.
fn parse<T>(json: &str) -> Result<HashMap<T::Id, T>, AssetError>
where
    T: for<'de> Deserialize<'de> + Identifiable,
    T::Id: Clone + Hash + Eq,
{
    let entries: Vec<T> = serde_json::from_str(json).map_err(AssetError::Parse)?;
    Ok(entries.into_iter().map(|e| (e.id().clone(), e)).collect())
}

fn load_asset<T>(path: &Path) -> Result<HashMap<T::Id, T>, AssetError>
where
    T: for<'de> Deserialize<'de> + Identifiable,
    T::Id: Clone + Hash + Eq,
{
    parse(&read_json(path)?)
}

/// Trait for assets that can be identified by a string ID.
trait Identifiable {
    type Id;
    fn id(&self) -> &Self::Id;
}

macro_rules! identifiable {
    ($($t:ty => $id:ty),* $(,)?) => {
        $(impl Identifiable for $t {
            type Id = $id;
            fn id(&self) -> &Self::Id {
                &self.id
            }
        })*
    }
}

identifiable!(
    ItemAsset => ItemId,
    WorkerAsset => WorkerId,
    BuildingAsset => BuildingId,
    CategoryAsset => CategoryId,
    RecipeAsset => RecipeId,
);

/// Creates newtype wrappers around `Arc<str>` for compile time distinction between asset IDs.
///
/// Asset IDs are frequently cloned as `HashMap` keys and stored in shared asset references,
/// but are never mutated.
///
/// Using `Arc<str>` makes cloning cheap by sharing the underlying allocation while avoiding
/// repeated heap allocations for identical strings.
///
/// # Example
///
/// ```
/// newtype!(FooId, BarId);
///
/// let foo = FooId::from("foo");
/// let bar: BarId = "bar".into();
/// ```
macro_rules! newtype {
    ($($name:ident),* $(,)?) => {
        $(
            #[derive(Debug, Clone, Hash, PartialEq, Eq)]
            pub struct $name(pub Arc<str>);

            impl From<&str> for $name {
                fn from(s: &str) -> Self {
                    Self(Arc::from(s))
                }
            }

            impl From<String> for $name {
                fn from(s: String) -> Self {
                    Self(Arc::from(s))
                }
            }

            impl<'de> Deserialize<'de> for $name {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    String::deserialize(deserializer).map(Self::from)
                }
            }

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    write!(f, "{}", &self.0)
                }
            }
        )*
    };
}

newtype!(ItemId, WorkerId, BuildingId, RecipeId, CategoryId);

pub type ItemList = Vec<ItemStack>;

#[derive(Debug, Clone, Deserialize)]
pub struct ItemStack {
    pub id: ItemId,
    pub count: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemAsset {
    pub id: ItemId,
    pub name: String,
    pub description: String,
    pub category: CategoryId,
    pub stack_limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkerAsset {
    pub id: WorkerId,
    pub name: String,
    pub description: String,
    pub category: CategoryId,
    pub stack_limit: usize,
    pub inventory_size: usize,
    pub speed: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BuildingAsset {
    pub id: BuildingId,
    pub name: String,
    pub description: String,
    pub base_cost: ItemList,
    pub cost_increase: f32,
    pub first_free: bool,
    pub tier_up_from: Option<BuildingId>,
    pub x_size: usize,
    pub y_size: usize,
    pub inventory_size: usize,
    pub recipes: Vec<RecipeId>,
    pub production_speed: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecipeAsset {
    pub id: RecipeId,
    pub inputs: ItemList,
    pub outputs: ItemList,
    pub time: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CategoryAsset {
    pub id: CategoryId,
    pub name: String,
    pub description: String,
}

#[derive(Debug)]
pub enum AssetError {
    Io(std::io::Error),
    Parse(serde_json::Error),
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AssetError::Io(err) => write!(f, "IO Error: {}", err),
            AssetError::Parse(err) => write!(f, "Parse Error: {}", err),
        }
    }
}

impl std::error::Error for AssetError {}

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
        let map: HashMap<ItemId, ItemAsset> = parse(json).unwrap();
        let item = map
            .get(&ItemId::from("iron_ore"))
            .expect("Iron Ore missing");
        assert_eq!(item.name, "Iron Ore");
        assert_eq!(item.category, "ore".into());
        assert_eq!(item.stack_limit, 256);
    }

    #[test]
    fn test_category() {
        let json = r#"[{
            "id": "worker",
            "name": "Worker",
            "description": "Carries items."
        }]"#;
        let map: HashMap<CategoryId, CategoryAsset> = parse(json).unwrap();
        let category = map
            .get(&CategoryId::from("worker"))
            .expect("Worker missing");
        assert_eq!(category.name, "Worker");
        assert_eq!(category.description, "Carries items.");
    }

    #[test]
    fn test_faulty_json() {
        let json = r#"[{
            "id": "bad",
            "name": "Bad"
        }]"#;
        let result: Result<HashMap<ItemId, ItemAsset>, AssetError> = parse(json);
        assert!(result.is_err());
    }
}
