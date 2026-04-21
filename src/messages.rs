use crate::aid::AID;
use crate::world_manager::Pos;


#[derive(Clone)]
pub enum Task{
    MoveTo(Pos),
    // senare:
    //TakeFrom{
        //inventory : AID<InventoryMessage>
        //item : Item,
        //amount: usize
    //},
    //GiveTo{

        //inventory: AID<InventoryMessage>,
        //item: Item,
        //amount: usize

    //},
    Idle,
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
