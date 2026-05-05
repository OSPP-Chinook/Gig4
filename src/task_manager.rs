use std::{
    collections::{HashMap, VecDeque},
    pin::Pin,
    sync::mpsc::Receiver,
};

use crate::{aid::AID, messages::EntityMessage, world_manager::Tile};
use crate::{
    item::Item,
    world_manager::{Pos, WorldGrid},
};

#[derive(Clone, PartialEq)]
pub enum Task {
    MoveTo(Pos),
    DeliverItem(Item, (AID<EntityMessage>, Pos), (AID<EntityMessage>, Pos)), //Deliver Item from A to B.
    Produce(usize),                                                          //produce recipe id
    Idle,
}

#[derive(Clone)]
pub enum TaskManagerMessage {
    GiveMeNewTask(AID<EntityMessage>), //Worker at pos A requests a new task
    GiveTaskTo(Task, AID<EntityMessage>), //Give some entity a task (if player wants a building to produce etc)
    CreatePath(Item, Pos, Pos),           //Create a path that delivers Item from A to B
    CreateMoveTask(Pos),
    Quit,
}

pub fn main(aid: AID<TaskManagerMessage>, mailbox: Receiver<TaskManagerMessage>, grid: WorldGrid) {
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
                let grid = &grid.lock().unwrap();
                let from_tile = grid.get(from.1).unwrap().get(from.0).unwrap().clone();
                let to_tile = grid.get(to.1).unwrap().get(to.0).unwrap().clone();
                if let Tile::Building(from_aid) = from_tile
                    && let Tile::Building(to_aid) = to_tile
                {
                    task_queue.push_back(Task::DeliverItem(item, (from_aid, from), (to_aid, to)));
                } else {
                }
            }

            TaskManagerMessage::CreateMoveTask(to) => {
                task_queue.push_back(Task::MoveTo(to));
            }

            TaskManagerMessage::Quit => {
                break;
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
        if let Task::MoveTo(pos) = prev_task {}
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

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc, Mutex,
            mpsc::{SendError, channel},
        },
        thread,
        time::Duration,
    };

    use crate::{task_manager, world_manager::Tile};

    use super::*;

    #[test]
    fn create_destroy() {
        let grid: WorldGrid = Arc::new(Mutex::new(vec![vec![Tile::Empty; 10]; 10]));
        let task_manager: AID<TaskManagerMessage> =
            AID::new(|aid, mailbox| main(aid, mailbox, grid));
        let _ = task_manager.send(TaskManagerMessage::Quit);
        thread::sleep(Duration::from_secs(1));
        //panic if can send message after quit
        let _ = task_manager
            .send(TaskManagerMessage::Quit)
            .inspect(|_| panic!());
    }

    //worker get idle when no tasks exist
    #[test]
    fn empty_task_queue() {
        let grid: WorldGrid = Arc::new(Mutex::new(vec![vec![Tile::Empty; 10]; 10]));
        let task_manager: AID<TaskManagerMessage> =
            AID::new(|aid, mailbox| main(aid, mailbox, grid));
        let (fake_worker, fake_worker_mailbox) = AID::<EntityMessage>::mock();
        let _ = task_manager.send(TaskManagerMessage::GiveMeNewTask(fake_worker.clone()));
        if let Ok(EntityMessage::Task(Task::Idle)) = fake_worker_mailbox.recv() {
        } else {
            panic!();
        }
    }

    ///task manager with one task gives back that one task when the assigned worker asks for a new task
    #[test]
    fn same_task_twice() {
        let grid: WorldGrid = Arc::new(Mutex::new(vec![vec![Tile::Empty; 10]; 10]));
        let task_manager: AID<TaskManagerMessage> =
            AID::new(|aid, mailbox| main(aid, mailbox, grid));
        let (fake_worker, fake_worker_mailbox) = AID::<EntityMessage>::mock();
        let _ = task_manager.send(TaskManagerMessage::CreateMoveTask((0, 0)));
        let _ = task_manager.send(TaskManagerMessage::GiveMeNewTask(fake_worker.clone()));
        if let Ok(EntityMessage::Task(Task::MoveTo(_))) = fake_worker_mailbox.recv() {
        } else {
            panic!("First")
        }
        let _ = task_manager.send(TaskManagerMessage::GiveMeNewTask(fake_worker.clone()));
        if let Ok(EntityMessage::Task(Task::MoveTo(_))) = fake_worker_mailbox.recv() {
        } else {
            panic!("Second")
        }
    }

    //Workers get idle when all tasks are occupied
    #[test]
    fn idle_when_no_available() {
        let grid: WorldGrid = Arc::new(Mutex::new(vec![vec![Tile::Empty; 10]; 10]));
        let task_manager: AID<TaskManagerMessage> =
            AID::new(|aid, mailbox| main(aid, mailbox, grid));
        let (fake_worker2, fake_worker_mailbox2) = AID::<EntityMessage>::mock();
        let (fake_worker, fake_worker_mailbox) = AID::<EntityMessage>::mock();
        let _ = task_manager.send(TaskManagerMessage::CreateMoveTask((0, 0)));
        let _ = task_manager.send(TaskManagerMessage::GiveMeNewTask(fake_worker.clone()));
        let _ = task_manager.send(TaskManagerMessage::GiveMeNewTask(fake_worker2.clone()));
        if let Ok(EntityMessage::Task(Task::MoveTo(_,))) = fake_worker_mailbox.recv() {
        } else {
            panic!("First")
        }
        if let Ok(EntityMessage::Task(Task::Idle)) = fake_worker_mailbox2.recv() {
        } else {
            panic!("Second")
        }
    }
}
