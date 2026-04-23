use crate::aid::AID;

pub type Task = ();

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
    TODO,
}

#[derive(Clone)]
pub enum TaskManagerMessage {}
