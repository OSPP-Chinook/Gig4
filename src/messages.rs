use std::sync::mpsc;

struct Actor {}
struct Task {}
type Pos = (usize, usize);

type WorldManagerMailbox =  mpsc::Receiver<(WorldManagerMessage, Actor)>;
type PlayerManagerMailbox =  mpsc::Receiver<(PlayerManagerMessage, Actor)>;
type TaskManagerMailbox =  mpsc::Receiver<(TaskManagerMessage, Actor)>;
type Entity =  mpsc::Receiver<(EntityMessage, Actor)>;



enum WorldManagerMessage {
    Move(Pos),
    TileInfo(Pos),
    KillMe,
    GetDisplay,
}

enum EntityMessage {
    Task(Task),
    KillYourself,
    Ok,
    Err
}

enum PlayerManagerMessage {
}

enum TaskManagerMessage {

}