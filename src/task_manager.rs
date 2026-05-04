use std::{
    collections::{HashMap, VecDeque}, pin::Pin, sync::mpsc::Receiver
};

use crate::{aid::AID, messages::EntityMessage};
use crate::item::Item;


type Pos = (usize, usize);

type RecipeId = usize;

struct Path {}

#[derive(Clone)]
pub enum Task {
    MoveTo(Pos),
    DeliverItem(Item, Pos, Pos), //Deliver Item from A to B.
    Produce(RecipeId),           //produce recipe id
    Idle,
}

#[derive(Clone)]
pub enum TaskManagerMessage {
    GiveMeNewTask(AID<EntityMessage>), //Worker at pos A requests a new task
    GiveTaskTo(Task, AID<EntityMessage>), //Give some entity a task (if player wants a building to produce etc)
    CreatePath(Item, Pos, Pos),           //Create a path that delivers Item from A to B
    CreateMoveTask(Pos),
}
pub fn main(aid: AID<TaskManagerMessage>, mailbox: Receiver<TaskManagerMessage>) {
    //Maps AID to assigned task
    let mut task_list: HashMap<AID<EntityMessage>, Task> = HashMap::new();
    //A queue of non-assigned tasks
    let mut task_queue: VecDeque<Task> = VecDeque::new();
    for msg in mailbox {
        match msg {
            TaskManagerMessage::GiveMeNewTask(aid) => {
                let _ = aid.send(EntityMessage::Task(assign_task(
                    aid.clone(),
                    &mut task_queue,
                    &mut task_list,
                )));
            }
            TaskManagerMessage::GiveTaskTo(task, to) => {
                let _ = to.send(EntityMessage::Task(task));
            }
            TaskManagerMessage::CreatePath(item, from, to) => {
                task_queue.push_back(Task::DeliverItem(item, from, to));
            }

            TaskManagerMessage::CreateMoveTask(to) => {
                task_queue.push_back(Task::MoveTo(to));
            }
        }
    }
}

//Gets a new tasks and updates queue and map accordingly, returns new task.
fn assign_task(
    aid: AID<EntityMessage>,
    task_queue: &mut VecDeque<Task>,
    task_list: &mut HashMap<AID<EntityMessage>, Task>,
) -> Task {
    
    //if had a task assigned previously
    if let Some(prev_task) = task_list.get(&aid) {
        if let Task::MoveTo(pos) = prev_task {
        }
        task_queue.push_back(prev_task.clone());
    }
    //if there are some new task available
    if let Some(new_task) = task_queue.pop_front() {
        task_list.insert(aid, new_task.clone());
        return new_task;
    } else {
        return Task::Idle;
    }
}
