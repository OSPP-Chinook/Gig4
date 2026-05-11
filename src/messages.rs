use crate::aid::AID;

use crate::inventory::InventoryMessage;
use crate::task_manager::Task;
use crate::world_manager::{Pos, Tile, WorldGrid};

#[derive(Clone)]
pub enum EntityMessage {
    Task(Task),
    KillYourself,
    Ok,
    Err,
    InventoryOk,
    InventoryErr,
    GetInventory(AID<EntityMessage>),
    SendInventory(AID<InventoryMessage>),
}

#[derive(Clone)]
pub enum PlayerManagerMessage {
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
