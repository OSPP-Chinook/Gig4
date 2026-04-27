use crate::{
    aid::AID,
    player_manager::WorldArray,
};

use crate::inventory::InventoryMessage;
use crate::item::Item;
use crate::world_manager::{Pos, Tile, WIDTH, HEIGHT};

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
    WorldUpdate(WorldArray),
    ShowTileInfo(Pos, Tile),
    TileNotFound(Pos),
    Notification(String), // if we ever want to notify the player of anything special
}

#[derive(Clone)]
pub enum TaskManagerMessage {
    AssignTask(Task),
    RevokeTask(Task),
    TaskDone(AID<EntityMessage>, Task),
}
