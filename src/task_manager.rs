use std::{
    collections::{HashMap, VecDeque},
    sync::mpsc::Receiver,
};

use crate::{
    aid::{AID, AIDHandle},
    worker::EntityMessage,
    assets::{ItemId, RecipeId},
    world_manager::{Pos, Tile, WorldGrid},
    zombie,
};

/// A task for a worker or building.
#[derive(Clone, PartialEq)]
pub enum Task {
    /// Instructs a worker to move to some position.
    MoveTo(Pos),
    /// Instructs a worker to deliver an item (specified by `ItemId`) from one building to another.
    DeliverItem(ItemId, (AID<EntityMessage>, Pos), (AID<EntityMessage>, Pos)),
    /// Instructs a building to produce according to a recipe, given by `RecipeId`.
    Produce(RecipeId),
    /// Instructs a building or worker to not do anything meaningful.
    Idle,
}

/// A message that the task manager can receive.
#[derive(Clone)]
pub enum TaskManagerMessage {
    /// A message sent by an entity that is about to die.
    /// 
    /// The task manager will forget the entity. It will also remove its tasks
    /// and any tasks that deliver items to or from that entity.
    KillMe(AID<EntityMessage>),
    /// A request from an entity to abandon its current task.
    RemoveMyTask(AID<EntityMessage>),
    /// A request from an entity to get a new task.
    /// 
    /// This is usually requested by workers.
    GiveMeNewTask(AID<EntityMessage>),
    /// A request to give a task to a specific entity.
    /// 
    /// This is usually requested by the player manager.
    GiveTaskTo(Task, AID<EntityMessage>),
    /// A request to schedule delivery of an item from one building to another.
    /// 
    /// The player manager sends this message when the player requests a delivery.
    CreatePath(ItemId, Pos, Pos),
    // /// A request to unschedule the delivery of an item from one building to another.

    // /// The player manager sends this message when the player requests to stop a delivery.
    // RemovePath(ItemId, Pos, Pos),
    /// A request to make a worker move to a particular position.
    CreateMoveTask(Pos),
    /// A request to quit the task manager, sent when quitting the game.
    Quit,
}

/// An error resulting from a request to schedule a task.
#[derive(Clone)]
pub enum TaskError {
    /// The task manager is dying (in a zombie state).
    ImDead,
}

/// The task manager's main loop, wherein the task manager processes incoming
/// messages and performs its logic.
fn main(mailbox: &Receiver<TaskManagerMessage>, grid: WorldGrid) {
    //Maps AID to assigned task
    let mut task_list: HashMap<AID<EntityMessage>, Task> = HashMap::new();
    //A queue of non-assigned tasks
    let mut task_queue: VecDeque<Task> = VecDeque::new();
    for msg in mailbox {
        match msg {
            TaskManagerMessage::KillMe(aid) => {
                // remove current task
                match task_list.remove(&aid) {
                    None | Some(Task::Idle | Task::Produce(_)) => {}
                    Some(task) => task_queue.push_back(task),
                }

                // remove tasks delivering to entity
                task_queue.retain(|task| match task {
                    Task::DeliverItem(_, (from, _), (to, _)) if *from == aid || *to == aid => false,
                    _ => true,
                });
            }
            TaskManagerMessage::RemoveMyTask(aid) => {
                task_list.remove(&aid);
            }
            TaskManagerMessage::GiveMeNewTask(aid) => {
                let _ = aid.send(EntityMessage::TaskResponse(Ok(assign_task(
                    aid.clone(),
                    &mut task_queue,
                    &mut task_list,
                ))));
            }
            TaskManagerMessage::GiveTaskTo(task, to) => {
                let _ = to.send(EntityMessage::TaskResponse(Ok(task)));
            }
            TaskManagerMessage::CreatePath(item, from, to) => {
                let grid = &grid.lock().unwrap();
                let from_tile = grid.get(from.1).unwrap().get(from.0).unwrap().clone();
                let to_tile = grid.get(to.1).unwrap().get(to.0).unwrap().clone();
                if let Tile::Building(from_aid, _) = from_tile
                    && let Tile::Building(to_aid, _) = to_tile
                {
                    task_queue.push_back(Task::DeliverItem(
                        item,
                        (from_aid.clone(), from),
                        (to_aid.clone(), to),
                    ));
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

/// Creates and launches a new joinable instance of the task manager.
pub fn new_joinable(grid: WorldGrid) -> (AID<TaskManagerMessage>, AIDHandle) {
    return AID::new_joinable(|aid, mailbox| {
        drop(aid);
        main(&mailbox, grid);

        zombie::task_manager_zombie(mailbox);
    });
}

/// Computes the next task to be assigned to a worker and updates the task
/// manager's internal state.
/// 
/// The task manager's internal table of assigned tasks (the `task_list`)
/// is keyed by `aid`, which is the AID of the assigned worker. If the worker
/// had a task previously assigned to it, that previous task will become
/// available for other workers to take, as this function places it in the
/// `task_queue`.
fn assign_task(
    aid: AID<EntityMessage>,
    task_queue: &mut VecDeque<Task>,
    task_list: &mut HashMap<AID<EntityMessage>, Task>,
) -> Task {
    //if had a task assigned previously
    if let Some(prev_task) = task_list.get(&aid) {
        task_queue.push_back(prev_task.clone());
    }
    //if there are some new task available
    if let Some(new_task) = task_queue.pop_front() {
        task_list.insert(aid, new_task.clone());
        new_task
    } else {
        Task::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{task_manager, world_manager::Tile};
    use std::sync::{Arc, Mutex};

    //worker get idle when no tasks exist
    #[test]
    fn empty_task_queue() {
        let grid: WorldGrid = Arc::new(Mutex::new(vec![vec![Tile::Empty; 10]; 10]));
        let task_manager = task_manager::new_joinable(grid).0;
        let (fake_worker, fake_worker_mailbox) = AID::<EntityMessage>::mock();
        let _ = task_manager.send(TaskManagerMessage::GiveMeNewTask(fake_worker.clone()));
        if let Ok(EntityMessage::TaskResponse(Ok(Task::Idle))) = fake_worker_mailbox.recv() {
        } else {
            panic!();
        }
    }

    ///task manager with one task gives back that one task when the assigned worker asks for a new task
    #[test]
    fn same_task_twice() {
        let grid: WorldGrid = Arc::new(Mutex::new(vec![vec![Tile::Empty; 10]; 10]));
        let task_manager = task_manager::new_joinable(grid).0;
        let (fake_worker, fake_worker_mailbox) = AID::<EntityMessage>::mock();
        let _ = task_manager.send(TaskManagerMessage::CreateMoveTask((0, 0)));
        let _ = task_manager.send(TaskManagerMessage::GiveMeNewTask(fake_worker.clone()));
        if let Ok(EntityMessage::TaskResponse(Ok(Task::MoveTo(_)))) = fake_worker_mailbox.recv() {
        } else {
            panic!("First")
        }
        let _ = task_manager.send(TaskManagerMessage::GiveMeNewTask(fake_worker.clone()));
        if let Ok(EntityMessage::TaskResponse(Ok(Task::MoveTo(_)))) = fake_worker_mailbox.recv() {
        } else {
            panic!("Second")
        }
    }

    //Workers get idle when all tasks are occupied
    #[test]
    fn idle_when_no_available() {
        let grid: WorldGrid = Arc::new(Mutex::new(vec![vec![Tile::Empty; 10]; 10]));
        let task_manager = task_manager::new_joinable(grid).0;
        let (fake_worker2, fake_worker_mailbox2) = AID::<EntityMessage>::mock();
        let (fake_worker, fake_worker_mailbox) = AID::<EntityMessage>::mock();
        let _ = task_manager.send(TaskManagerMessage::CreateMoveTask((0, 0)));
        let _ = task_manager.send(TaskManagerMessage::GiveMeNewTask(fake_worker.clone()));
        let _ = task_manager.send(TaskManagerMessage::GiveMeNewTask(fake_worker2.clone()));
        if let Ok(EntityMessage::TaskResponse(Ok(Task::MoveTo(_)))) = fake_worker_mailbox.recv() {
        } else {
            panic!("First")
        }
        if let Ok(EntityMessage::TaskResponse(Ok(Task::Idle))) = fake_worker_mailbox2.recv() {
        } else {
            panic!("Second")
        }
    }
}
