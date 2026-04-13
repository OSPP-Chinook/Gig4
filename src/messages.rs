use crate::aid::AID;

pub type Task = ();
pub type Pos = (usize, usize);

#[derive(Clone)]
pub enum WorldManagerMessage {
    Stop, // is only necessary if there are circular AIDs (which there probably will be)
    Move(Pos, AID<EntityMessage>),
    KillMe(AID<EntityMessage>),
    TileInfo(Pos, AID<PlayerManagerMessage>),
    GetDisplay(AID<PlayerManagerMessage>),
}

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
