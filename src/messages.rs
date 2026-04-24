use crate::{
    aid::AID,
    player_manager::WorldArray,
};

use crate::inventory::InventoryMessage;
use crate::item::Item;
use crate::world_manager::Pos;

#[derive(Clone)]
pub enum Task {
    MoveTo(Pos),

    AddItem {
        item: Item,
        amount: usize,
    },

    RemoveItem {
        item: Item,
        amount: usize,
    },

    TakeFrom {
        from: AID<InventoryMessage>,
        item: Item,
        amount: usize,
    },

    GiveTo {
        to: AID<InventoryMessage>,
        item: Item,
        amount: usize,
    },

    PrintInventory(String),

    Idle,
}

#[derive(Clone)]
pub enum EntityMessage {
    Task(Task),
    KillYourself,
    Ok,
    Err,
    InventoryOk,
    InventoryErr,
}

#[derive(Clone)]
pub enum PlayerManagerMessage {
    TODO(WorldArray),
}

#[derive(Clone)]
pub enum TaskManagerMessage {
    AssignTask(Task),
}
