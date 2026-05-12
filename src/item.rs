use std::{hash::Hash};

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum Item {
    Mutexium,
    Semaphorite,
    Actorisite,
}

impl Item {
    pub fn to_str(&self) -> &str {
        match self {
            Item::Mutexium => return "Mutexium",
            Item::Semaphorite => return "Semaphorite",
            Item::Actorisite => return "Actorisite",
        };
    }
}