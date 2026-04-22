use std::{sync::mpsc::Receiver};

use crate::{aid::AID, messages::EntityMessage};

#[derive(Clone)]
enum Item {}
type Pos = (usize, usize);

type RecipeId = usize;

struct Path {}

#[derive(Clone)]
pub enum Task {
    DeliverItem((Item), Pos, Pos), //Deliver Item from A to B.
    Produce(RecipeId),             //produce recipe id
    Idle,
}

#[derive(Clone)]
pub enum TaskManagerMessage {
    GiveMeNewTask(AID<EntityMessage>, Pos), //Worker at pos A requests a new task
    GiveTaskTo(Task, AID<EntityMessage>), //Give some entity a task (if player wants a building to produce etc)
    CreatePath(Item, Pos, Pos),           //Create a path that delivers Item from A to B
}
fn main(aid: AID<TaskManagerMessage>, mailbox: Receiver<TaskManagerMessage>) {
    for msg in mailbox {
        match msg {
            TaskManagerMessage::GiveMeNewTask(x, y) => (),
            TaskManagerMessage::GiveTaskTo(task, to) => _ = to.send(EntityMessage::Task(task)),
            TaskManagerMessage::CreatePath(item, from, to) => (),
        }
    }
}