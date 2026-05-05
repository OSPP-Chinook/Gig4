use crate::aid::AID;
use crate::inventory::{self, InventoryMessage};
use crate::item::Item;
use crate::messages::EntityMessage;
use crate::task_manager::{Task, TaskManagerMessage};
use crate::world_manager::{Pos, WorldManagerMessage};
use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;

/// Ren logik- och state för en entity.
///
/// EntityCore ansvarar för:
/// - att hålla nuvarande position (`current_pos`)
/// - att lagra en väntande flytt (`pending_move`)
/// - att behandla inkommande tasks
/// - att uppdatera state baserat på Ok/Err från WorldManager
///
/// Innehåller ingen actor‑logik.
/// Används av `Entity` som den faktiska logikdelen.
#[allow(dead_code)]
struct EntityCore {
    current_pos: Pos,
    pending_move: Option<Pos>,
    sub_tasks: VecDeque<SubTask>,
}

enum SubTask {
    Move(Pos),
    TakeItem(Item),
    GiveItem(Item),
}

enum Request {
    Move(Pos),
    RequestTask,
    GiveItem(Item),
    TakeItem(Item),
}

#[allow(dead_code)]
impl EntityCore {
    // skapar en EntityCore  med given start position
    fn new(start_pos: Pos) -> EntityCore {
        EntityCore {
            current_pos: start_pos,
            pending_move: None,
            sub_tasks: VecDeque::new(),
        }
    }

    fn process_move(&mut self, pos: Pos) -> Option<Pos> {
        let mut return_pos: Option<Pos> = None;

        if pos.0 > self.current_pos.0 {
            return_pos = Some((self.current_pos.0 + 1, self.current_pos.1));
        } else if pos.0 < self.current_pos.0 {
            return_pos = Some((self.current_pos.0 - 1, self.current_pos.1));
        } else if pos.1 > self.current_pos.1 {
            return_pos = Some((self.current_pos.0, self.current_pos.1 + 1));
        } else if pos.1 < self.current_pos.1 {
            return_pos = Some((self.current_pos.0, self.current_pos.1 - 1));
        } else {
            self.sub_tasks.pop_front();
        }
        self.pending_move = return_pos;
        return return_pos;
    }

    fn process_task(&mut self) -> Option<Request> {
        if self.sub_tasks.is_empty() {
            return Some(Request::RequestTask);
        }
        if let Some(sub_task) = self.sub_tasks.front() {
            match sub_task {
                SubTask::Move(pos) => {
                    if let Some(target) = self.process_move(*pos) {
                        return Some(Request::Move(target));
                    } else {
                        //
                        return None;
                    }
                }
                SubTask::GiveItem(item) => {
                    return Some(Request::GiveItem(*item));
                }
                SubTask::TakeItem(item) => {
                    return Some(Request::TakeItem(*item));
                }
            }
        }
        return None;
    }

    /// Behandlar en Task och returnerar eventuell Move-position
    /// som Entity-aktorn ska skicka till WorldManager.
    fn new_task(&mut self, task: Task) {   
        match task {
            Task::MoveTo(pos) => {
                self.sub_tasks.push_back(SubTask::Move(pos));
            }
            Task::DeliverItem(item,from, to) => {
                self.sub_tasks.push_back(SubTask::Move(from));
                self.sub_tasks.push_back(SubTask::TakeItem(item));
                self.sub_tasks.push_back(SubTask::Move(to));
                self.sub_tasks.push_back(SubTask::GiveItem(item));
            },
            _ => (),
            // Task::AddItem { .. } => {
            //     self.is_busy = true;
            //     None
            // }
            // Task::RemoveItem { .. } => {
            //     self.is_busy = true;
            //     None
            // }
            // Task::TakeFrom { .. } => {
            //     self.is_busy = true;
            //     None
            // }
            // Task::GiveTo { .. } => {
            //     self.is_busy = true;
            //     None
            // }
            // Task::PrintInventory(_) => {
            //     self.is_busy = true;
            //     None
        }
    }
    /// Anropas när WorldManager godkänner en flytt.
    /// Uppdaterar current_pos och tömmer pending_move.
    fn apply_ok(&mut self) {
        if let Some(pos) = self.pending_move.take() {
            self.current_pos = pos;
            self.pending_move = None;
        }
    }
    /// Anropas när WorldManager nekar en flytt.
    /// Tömmer pending_move utan att ändra current_pos.
    fn apply_err(&mut self) {
        self.pending_move = None;
    }
}

/// Actor som representerar en Entity i världen.
///
/// Entity ansvarar för:
/// - att ta emot `EntityMessage`
/// - att vidarebefordra tasks till `EntityCore`
/// - att skicka `WorldManagerMessage::Move` när core signalerar en flytt
/// - att uppdatera core-state baserat på Ok/Err från WorldManager
///
/// All logik ligger i `EntityCore`.  
/// Entity själv hanterar endast actor‑beteende och message‑flow.
pub struct Entity {
    core: EntityCore,
    waiting: bool,
    world_aid: AID<WorldManagerMessage>,
    task_aid: AID<TaskManagerMessage>,
    inventory: AID<InventoryMessage>,
    self_aid: AID<EntityMessage>,
}

impl Entity {
    pub fn new(
        world: AID<WorldManagerMessage>,
        task: AID<TaskManagerMessage>,
        start_pos: Pos,
    ) -> AID<EntityMessage> {
        AID::new(move |aid, mailbox| {
            let mut entity = Entity::create(aid.clone(), world, task, start_pos);

            entity.run(mailbox);
        })
    }

    fn create(
        self_aid: AID<EntityMessage>,
        world: AID<WorldManagerMessage>,
        task: AID<TaskManagerMessage>,
        start_pos: Pos,
    ) -> Self {
        Entity {
            core: EntityCore::new(start_pos),
            waiting: false,
            world_aid: world,
            task_aid: task,
            inventory: inventory::init(),
            self_aid: self_aid,
        }
    }

    fn run(&mut self, mailbox: Receiver<EntityMessage>) {
        loop {
            while let Ok(msg) = mailbox.try_recv() {
                match msg {
                    EntityMessage::Task(task) => {
                        self.core.new_task(task);
                        self.waiting = false;
                    },

                    EntityMessage::KillYourself => {
                        let _ = self
                            .world_aid
                            .send(WorldManagerMessage::KillMe(self.self_aid.clone()));
                        break;
                    }

                    EntityMessage::Ok => {
                        //world manager godkände flyyten
                        //uppdatera EntityCore-> cunnrent_pos
                        self.core.apply_ok();
                        thread::sleep(Duration::from_millis(500));
                        self.waiting = false;
                    }

                    EntityMessage::Err => {
                        // world manager neckade flytten
                        // ingen ändring i pos
                        self.core.apply_err();
                        self.waiting = false;
                    }

                    EntityMessage::InventoryOk => {
                        self.waiting = false;
                    }
                    
                    EntityMessage::InventoryErr => {
                        self.waiting = false;
                        //tillfälligt lösning
                    }
                }
            }
            if self.waiting {
                continue;
            }
            //process task
            if let Some(req) = self.core.process_task() {
                match req {
                    Request::Move(pos) => {
                        let _ = self
                            .world_aid
                            .send(WorldManagerMessage::Move(pos, self.self_aid.clone()));
                        self.waiting = true;
                    }
                    Request::RequestTask => {
                        let _ = self
                            .task_aid
                            .send(TaskManagerMessage::GiveMeNewTask(self.self_aid.clone()));
                        self.waiting = true;
                    }
                    Request::GiveItem(item) => {
                        thread::sleep(Duration::from_millis(3000));
                        //println!("Dropped 1000 Megaforium");
                        self.core.sub_tasks.pop_front();
                        self.waiting = false;
                    }
                    Request::TakeItem(item) => {
                        thread::sleep(Duration::from_millis(3000));
                        //println!("Took 1000 Megaforium");
                        self.core.sub_tasks.pop_front();
                        self.waiting = false;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::messages;

    use super::*;

    #[test]
    fn apply_task() {
        let start_pos = (1, 1);
        let mut core = EntityCore::new(start_pos);

        let new_pos = (10, 10);
        let task = Task::MoveTo(new_pos);

        //assert_eq!(core.apply_task(task), Some(new_pos));
        //assert_eq!(core.pending_move, Some(new_pos));
    }

    #[test]
    fn apply_ok() {
        let start_pos = (1, 1);
        let mut core = EntityCore::new(start_pos);

        let new_pos = (20, 20);
        let task = Task::MoveTo(new_pos);
        //core.apply_task(task);
        core.apply_ok();
        assert_eq!(core.current_pos, new_pos);
    }

    #[test]
    fn apply_err() {
        let start_pos = (1, 1);
        let mut core = EntityCore::new(start_pos);

        let new_pos = (3, 8);

        let task = Task::MoveTo(new_pos);

        //core.apply_task(task);
        core.apply_err();

        assert_eq!(core.current_pos, start_pos);
        assert_eq!(core.pending_move, None);
    }

    #[test]

    fn is_bussy() {
        let start_pos = (10, 10);
        let mut core = EntityCore::new(start_pos);
        //assert_eq!(core.is_busy, false);

        let new_pos = (20, 20);
        let task = Task::MoveTo(new_pos);
        //core.apply_task(task);

        //assert_eq!(core.is_busy, true);
    }
}
