use crate::aid::AID;

use crate::task_manager::Task;
use crate::world_manager::{WorldGrid, Pos, Tile};

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
    WorldUpdate(WorldGrid),
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
