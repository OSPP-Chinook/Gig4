use crate::aid::AID;
use crate::inventory::{self, InventoryMessage};
use crate::item::Item;
use crate::messages::EntityMessage;
use crate::task_manager::{Task, TaskManagerMessage};
use crate::world_manager::{Pos, WorldManagerMessage};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;

// duration to wait after moving
const MOVE_TIME: Duration = Duration::from_millis(250);
// duration to wait after transferring items
const TRANSFER_TIME: Duration = Duration::from_millis(1500);

/// Ren logik- och state för en entity.
///
/// EntityCore ansvarar för:
/// - att hålla nuvarande position (`current_pos`)
/// - att lagra en väntande flytt (`pending_move`)
/// - att behandla inkommande tasks
/// - att lagra state för pathfinding
/// - att uppdatera state baserat på Ok/Err från WorldManager
///
/// Innehåller ingen actor‑logik.
/// Används av `Entity` som den faktiska logikdelen.
#[allow(dead_code)]
struct EntityCore {
    current_pos: Pos,
    pending_move: Option<Pos>,
    sub_tasks: VecDeque<SubTask>,
    open_neighbors: HashSet<Pos>,
    heuristic: HashMap<Pos, usize>,
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

// nicer to keep it squared to avaoid roots and floats
fn euclidean_distance_squared(from: Pos, to: Pos) -> usize {
    return from.0.abs_diff(to.0).pow(2) + from.1.abs_diff(to.1).pow(2);
}

// Returns a set of all adjacent positions
fn neighbors(pos: Pos) -> HashSet<Pos> {
    let mut set = HashSet::new();
    set.insert((pos.0 + 1, pos.1));
    set.insert((pos.0, pos.1 + 1));
    if pos.0 > 0 {
        set.insert((pos.0 - 1, pos.1));
    }
    if pos.1 > 0 {
        set.insert((pos.0, pos.1 - 1));
    }
    return set;
}

#[allow(dead_code)]
impl EntityCore {
    // skapar en EntityCore  med given start position
    fn new(start_pos: Pos) -> EntityCore {
        EntityCore {
            current_pos: start_pos,
            pending_move: None,
            sub_tasks: VecDeque::new(),
            open_neighbors: neighbors(start_pos),
            heuristic: HashMap::new(),
        }
    }

    fn pathfind(&mut self, dst: Pos) -> Option<Pos> {
        // RTA* algorithm

        if self.current_pos == dst {
            return None;
        }

        let mut best: Option<(Pos, usize)> = None;
        let mut second: Option<(Pos, usize)> = None;

        for &s in self.open_neighbors.iter() {
            // In my experience, using a euclidean distance heuristic
            // produces better looking paths, even on discrete grids
            // but this could be changed to manhattan distance as well.
            let default = euclidean_distance_squared(s, dst);
            let h_s = *self.heuristic.get(&s).unwrap_or(&default);
            // all neighbors are a distance 1 away
            let f_s = h_s.checked_add(1).unwrap_or(h_s);

            // find best and second-best neighbor
            if best.is_none_or(|(_, f)| f > f_s) {
                second = best;
                best = Some((s, f_s));
            } else if second.is_none_or(|(_, f)| f > f_s) {
                second = Some((s, f_s));
            }
        }

        // second best defaults to infinity
        let f_new = second.map(|(_, f)| f).unwrap_or(usize::MAX);
        // update current heuristic to and second best neighbor
        // The intuition is that revisiting this node is only as
        // good as taking the second best path since the best path
        // will have already been expored and not worked out.
        // It should also not decrease since visiting it again
        // shouldn't make it better.
        if self
            .heuristic
            .get(&self.current_pos)
            .is_none_or(|&f| f < f_new)
        {
            self.heuristic.insert(self.current_pos, f_new);
        }

        // try to take best path
        return best.map(|(pos, _)| pos);
    }

    fn process_task(&mut self) -> Option<Request> {
        if let Some(sub_task) = self.sub_tasks.front() {
            match sub_task {
                SubTask::Move(pos) => {
                    if *pos == self.current_pos {
                        self.sub_tasks.pop_front();
                        self.heuristic.clear();
                        return None;
                    }

                    if let Some(target) = self.pathfind(*pos) {
                        self.pending_move = Some(target);
                        return Some(Request::Move(target));
                    } else {
                        // completely stuck
                        // wait a bit and hope something has moved until next iteration
                        thread::sleep(MOVE_TIME);
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
        } else {
            return Some(Request::RequestTask);
        }
    }

    /// Behandlar en Task och returnerar eventuell Move-position
    /// som Entity-aktorn ska skicka till WorldManager.
    fn new_task(&mut self, task: Task) {
        match task {
            Task::MoveTo(pos) => {
                self.sub_tasks.push_back(SubTask::Move(pos));
            }
            Task::DeliverItem(item, from, to) => {
                self.sub_tasks.push_back(SubTask::Move(from));
                self.sub_tasks.push_back(SubTask::TakeItem(item));
                self.sub_tasks.push_back(SubTask::Move(to));
                self.sub_tasks.push_back(SubTask::GiveItem(item));
            }
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
            // recalculate neighbors
            self.open_neighbors = neighbors(pos);
        }
    }
    /// Anropas när WorldManager nekar en flytt.
    /// Tömmer pending_move utan att ändra current_pos.
    fn apply_err(&mut self) {
        if let Some(pos) = self.pending_move.take() {
            // pos is not open
            self.open_neighbors.remove(&pos);
        }
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
    alive: bool,
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
            alive: true,
            waiting: false,
            world_aid: world,
            task_aid: task,
            inventory: inventory::init(),
            self_aid: self_aid,
        }
    }

    fn msg_handler(&mut self, msg: EntityMessage) {
        match msg {
            EntityMessage::Task(task) => {
                self.core.new_task(task);
                self.waiting = false;
            }

            EntityMessage::KillYourself => {
                let _ = self
                    .world_aid
                    .send(WorldManagerMessage::KillMe(self.self_aid.clone()));
                self.alive = false;
            }

            EntityMessage::Ok => {
                //world manager godkände flyyten
                //uppdatera EntityCore-> cunnrent_pos
                self.core.apply_ok();
                self.waiting = false;
                thread::sleep(MOVE_TIME);
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

    fn run(&mut self, mailbox: Receiver<EntityMessage>) {
        loop {
            while self.waiting {
                if let Ok(msg) = mailbox.recv() {
                    self.msg_handler(msg);
                }
            }
            while let Ok(msg) = mailbox.try_recv() {
                self.msg_handler(msg);
            }

            if !self.alive {
                break;
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
                        thread::sleep(TRANSFER_TIME);
                        //println!("Dropped 1000 Megaforium");
                        self.core.sub_tasks.pop_front();
                        self.waiting = false;
                    }
                    Request::TakeItem(item) => {
                        thread::sleep(TRANSFER_TIME);
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
