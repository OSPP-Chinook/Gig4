use crate::{
    aid::AID,
    player_manager::WorldArray,
};


pub type Task = ();

#[derive(Clone)]
pub enum EntityMessage {
    Task(Task),
    KillYourself,
    Ok,
    Err,
}

#[derive(Clone)]
pub enum PlayerManagerMessage {
    TODO,
}

#[derive(Clone)]
pub enum TaskManagerMessage {}
