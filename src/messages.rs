use crate::aid::AID;
use crate::inventory::InventoryMessage;
use crate::task_manager::Task;
use crate::world_manager::{Pos, Tile};

#[derive(Clone)]
pub enum GetInventoryError {
    ImDead,
    ImWorker,
}

#[derive(Clone)]
pub enum ItemTransferError {
    ImDead,
    TheyreDead,
    RecipeChange,
    InsufficientItems,
    TooManyItems,
}

#[derive(Clone)]
pub enum MoveError {
    ImDead,
    Occupied(Pos),
}

#[derive(Clone)]
pub enum TaskError {
    ImDead,
}

#[derive(Clone)]
pub enum EntityMessage {
    // sent by world manager spontaneously
    KillYourself,

    // sent by timer responding to start_timer
    TimerResponse,

    // sent by worker to building spontaneously
    GetInventory(AID<EntityMessage>),

    // sent by building to worker responding to GetInventory
    GetInventoryResponse(Result<AID<InventoryMessage>, GetInventoryError>),

    // sent by inventory responding to Add/Remove/GiveTo/TakeFrom
    ItemTransferResponse(Result<(), ItemTransferError>),

    // sent by world manager to worker responding to Move
    MoveResponse(Result<Pos, MoveError>),

    // sent by task manager respondong to GiveMeNewTask
    TaskResponse(Result<Task, TaskError>),

    // sent by player manager spontaneously
    FetchInventoryStatus(AID<PlayerManagerMessage>),

    // sent by player manager spontaneously
    FetchCurrentTask(AID<PlayerManagerMessage>),

    // sent by world manager spontaneously
    Pause,

    // sent by world manager spontaneously
    Unpause,
}

#[derive(Clone)]
pub enum PlayerManagerMessage {
    Quit,
    ShowTileInfo(Pos, Tile),
    TileNotFound(Pos),
    Notification(String), // if we ever want to notify the player of anything special
    InventoryStatusResult(Option<String>),
    CurrentTaskResult(Option<Task>),
}
