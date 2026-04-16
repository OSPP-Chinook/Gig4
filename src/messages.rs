use std::sync::mpsc;

struct Actor {}
struct Task {}

type Pos = (usize, usize);

type WorldManagerMailbox = mpsc::Receiver<(WorldManagerMessage, Actor)>;
type PlayerManagerMailbox = mpsc::Receiver<(PlayerManagerMessage, Actor)>;
type TaskManagerMailbox = mpsc::Receiver<(TaskManagerMessage, Actor)>;
type EntityMailbox = mpsc::Receiver<(EntityMessage, Actor)>;

pub enum WorldManagerMessage {
    Move(Pos),
    TileInfo(Pos),
    KillMe,
    GetDisplay,
}

pub enum EntityMessage {
    Task(Task),
    KillYourself,
    Ok,
    Err
}

pub enum PlayerManagerMessage {

}

pub enum TaskManagerMessage {

}
