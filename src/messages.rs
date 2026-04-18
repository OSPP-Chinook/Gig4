use crate::aid::AID;
use crate::world_manager::Pos;


#[derive(Clone)]
pub enum Task{
    MoveTo(Pos),
    Idle,
    // senare:
    // Gather,
    // Deliver
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
