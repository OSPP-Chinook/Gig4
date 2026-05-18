use crate::aid::{AID, AIDHandle};
use crate::assets::{Assets, ItemId, ItemStack, WorkerId};
use crate::inventory::{self, InventoryMessage};
use crate::messages::{
    EntityMessage, GetInventoryError, ItemTransferError, MoveError, PlayerManagerMessage, TaskError,
};
use crate::task_manager::{Task, TaskManagerMessage};
use crate::world_manager::{Pos, WorldManagerMessage};
use crate::zombie;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;

// duration to wait while idling
const IDLE_TIME: Duration = Duration::from_millis(500);
// duration to wait after moving
const MOVE_TIME: Duration = Duration::from_millis(250);
// duration to wait after transferring items
const TRANSFER_TIME: Duration = Duration::from_millis(5000);

/// Ren logik- och state för en worker.
///
/// WorkerCore ansvarar för:
/// - att hålla nuvarande position (`current_pos`)
/// - att lagra en väntande flytt (`pending_move`)
/// - att behandla inkommande tasks
/// - att lagra state för pathfinding
/// - att uppdatera state baserat på Ok/Err från WorldManager
///
/// Innehåller ingen actor‑logik.
/// Används av `Worker` som den faktiska logikdelen.
#[allow(dead_code)]
struct WorkerCore {
    current_pos: Pos,
    current_task: Task,
    sub_tasks: VecDeque<SubTask>,
    open_neighbors: HashSet<Pos>,
    heuristic: HashMap<Pos, usize>,
}

#[derive(Clone)]
enum SubTask {
    Idle,
    Move(Pos),
    TakeItem(AID<EntityMessage>, ItemId),
    GiveItem(AID<EntityMessage>, ItemId),
    Done,
}

fn manhattan_distance(from: Pos, to: Pos) -> usize {
    return from.0.abs_diff(to.0) + from.1.abs_diff(to.1);
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
impl WorkerCore {
    // skapar en WorkerCore med given start position
    fn new(start_pos: Pos) -> WorkerCore {
        WorkerCore {
            current_pos: start_pos,
            current_task: Task::Idle,
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

    fn process_task(&mut self) -> SubTask {
        if self.sub_tasks.is_empty() {
            self.current_task = Task::Idle;
            return SubTask::Done;
        }
        let sub_task = self.sub_tasks.front().unwrap();
        if let SubTask::Move(pos) = sub_task {
            // check if adjecant to goal
            if manhattan_distance(self.current_pos, *pos) <= 1 {
                self.sub_tasks.pop_front();
                self.heuristic.clear();
                return self.process_task();
            }

            if let Some(target) = self.pathfind(*pos) {
                return SubTask::Move(target);
            } else {
                // completely stuck
                // wait and hope something moves out of the way
                return SubTask::Idle;
            }
        }
        return sub_task.clone();
    }

    /// Behandlar en Task och returnerar eventuell Move-position
    /// som Worker-aktorn ska skicka till WorldManager.
    fn new_task(&mut self, task: Task) {
        match task {
            Task::MoveTo(pos) => {
                self.sub_tasks.push_back(SubTask::Move(pos));
                self.current_task = Task::MoveTo(pos);
            }
            Task::DeliverItem(item, (from_aid, from), (to_aid, to)) => {
                self.sub_tasks.push_back(SubTask::Move(from));
                self.sub_tasks
                    .push_back(SubTask::TakeItem(from_aid.clone(), item.clone()));
                self.sub_tasks.push_back(SubTask::Move(to));
                self.sub_tasks
                    .push_back(SubTask::GiveItem(to_aid.clone(), item.clone()));
                self.current_task = Task::DeliverItem(item, (from_aid, from), (to_aid, to));
            }
            Task::Idle => {
                self.sub_tasks.clear();
                self.current_task = Task::Idle;
            }
            Task::Produce(_) => {} // shouldn't happen
        }
    }
    /// Anropas när WorldManager godkänner en flytt.
    /// Uppdaterar current_pos och open_neighbors.
    fn apply_ok(&mut self, pos: Pos) {
        self.current_pos = pos;
        // open all neighbors
        self.open_neighbors = neighbors(pos);
    }
    /// Anropas när WorldManager nekar en flytt.
    /// Uppdaterar open_neighbors.
    fn apply_err(&mut self, pos: Pos) {
        // pos is not open
        self.open_neighbors.remove(&pos);
    }
}

/// Actor som representerar en Worker i världen.
///
/// Worker ansvarar för:
/// - att ta emot `EntityMessage`
/// - att vidarebefordra tasks till `WorkerCore`
/// - att skicka `WorldManagerMessage::Move` när core signalerar en flytt
/// - att uppdatera core-state baserat på Ok/Err från WorldManager
///
/// All logik ligger i `WorkerCore`.  
/// Worker själv hanterar endast actor‑beteende och message‑flow.
pub struct Worker {
    core: WorkerCore,
    alive: bool,
    paused: bool,
    waiting: bool,
    pending_inventory_task: Option<(bool, ItemId)>,
    world_aid: AID<WorldManagerMessage>,
    task_aid: AID<TaskManagerMessage>,
    inventory: AID<InventoryMessage>,
    self_aid: AID<EntityMessage>,
    assets: Arc<Assets>,
    id: WorkerId,
}

impl Worker {
    pub fn new(
        world: AID<WorldManagerMessage>,
        task: AID<TaskManagerMessage>,
        start_pos: Pos,
        assets: Arc<Assets>,
        id: WorkerId,
    ) -> AID<EntityMessage> {
        Worker::new_joinable(world, task, start_pos, assets, id).0
    }

    pub fn new_joinable(
        world: AID<WorldManagerMessage>,
        task: AID<TaskManagerMessage>,
        start_pos: Pos,
        assets: Arc<Assets>,
        id: WorkerId,
    ) -> (AID<EntityMessage>, AIDHandle) {
        AID::new_joinable(move |aid, mailbox| {
            let mut worker = Worker::create(aid, world, task, start_pos, assets, id);
            worker.run(&mailbox);

            worker.destroy();
            zombie::entity_zombie(mailbox);
        })
    }

    fn create(
        self_aid: AID<EntityMessage>,
        world: AID<WorldManagerMessage>,
        task: AID<TaskManagerMessage>,
        start_pos: Pos,
        assets: Arc<Assets>,
        id: WorkerId,
    ) -> Self {
        let inventory_size = assets.workers.get(&id).unwrap().inventory_size;

        Worker {
            core: WorkerCore::new(start_pos),
            alive: true,
            paused: false,
            waiting: false,
            pending_inventory_task: None,
            world_aid: world,
            task_aid: task,
            inventory: inventory::init(assets.clone(), inventory_size),
            self_aid: self_aid,
            assets,
            id,
        }
    }

    fn destroy(self) {
        let _ = self.inventory.send(InventoryMessage::KillYourself);
        let _ = self
            .task_aid
            .send(TaskManagerMessage::KillMe(self.self_aid.clone()));
        drop(self);
    }

    fn invalid_task(&mut self) {
        self.core.current_task = Task::Idle;
        self.core.sub_tasks.clear();
        self.waiting = false;
        let _ = self
            .task_aid
            .send(TaskManagerMessage::RemoveMyTask(self.self_aid.clone()));
    }

    fn msg_handler(&mut self, msg: EntityMessage) {
        match msg {
            EntityMessage::KillYourself => {
                self.alive = false;
            }

            EntityMessage::TaskResponse(res) => match res {
                Ok(task) => {
                    self.core.new_task(task);
                    self.waiting = false;
                }
                Err(TaskError::ImDead) => {} // should receive KillYourself shortly
            },

            EntityMessage::MoveResponse(res) => match res {
                Ok(pos) => {
                    //world manager godkände flyyten
                    //uppdatera WorkerCore-> current_pos
                    self.core.apply_ok(pos);
                    self.waiting = false;

                    let speed = self.assets.workers.get(&self.id).unwrap().speed;
                    let time = Duration::from_secs_f32(MOVE_TIME.as_secs_f32() / speed);
                    thread::sleep(time);
                }
                Err(MoveError::Occupied(pos)) => {
                    // world manager neckade flytten
                    // ingen ändring i pos
                    self.core.apply_err(pos);
                    self.waiting = false;
                }
                Err(MoveError::ImDead) => {} // should receive KillYourself shortly
            },

            EntityMessage::ItemTransferResponse(res) => match res {
                Ok(()) => {
                    self.core.sub_tasks.pop_front();
                    self.pending_inventory_task = None;
                    self.waiting = false;
                }
                Err(ItemTransferError::InsufficientItems | ItemTransferError::TooManyItems) => {
                    self.pending_inventory_task = None;
                    self.waiting = false;
                }
                Err(ItemTransferError::RecipeChange | ItemTransferError::TheyreDead) => {
                    self.invalid_task()
                }
                Err(ItemTransferError::ImDead) => self.alive = false, // something has gone very wrong
            },

            EntityMessage::GetInventory(aid) => {
                // workers should't need to transfer items between eachother
                let _ = aid.send(EntityMessage::GetInventoryResponse(Err(
                    GetInventoryError::ImWorker,
                )));
            }

            EntityMessage::GetInventoryResponse(res) => match res {
                Ok(inventory) => {
                    if let Some((send, item)) = self.pending_inventory_task.clone() {
                        if send {
                            let _ = self.inventory.send(InventoryMessage::GiveTo(
                                self.self_aid.clone(),
                                inventory,
                                vec![ItemStack::new(item, 10)],
                            ));
                        } else {
                            let _ = self.inventory.send(InventoryMessage::TakeFrom(
                                self.self_aid.clone(),
                                inventory,
                                vec![ItemStack::new(item, 10)],
                            ));
                        }
                    }
                }
                Err(GetInventoryError::ImWorker | GetInventoryError::ImDead) => self.invalid_task(),
            },

            EntityMessage::FetchInventoryStatus(pm_aid) => {
                _ = self.inventory.send(InventoryMessage::GiveStatus(pm_aid));
            }

            EntityMessage::FetchCurrentTask(pm_aid) => {
                _ = pm_aid.send(PlayerManagerMessage::CurrentTaskResult(Some(
                    self.core.current_task.clone(),
                )));
            }

            EntityMessage::Pause => {
                self.paused = true;
            }
            
            EntityMessage::Unpause => {
                self.paused = false;
            }
        }
    }

    fn run(&mut self, mailbox: &Receiver<EntityMessage>) {
        let mut pause_messages: Vec<EntityMessage> = vec![];
        'outer: loop {
            while self.paused {
                if let Ok(msg) = mailbox.recv() {
                    if let EntityMessage::Unpause = msg {
                        self.paused = false;
                        //send back all messages received while paused, could also send back 
                        //directly but then it would constantly read messages and never sleep
                        while let Some(pause_msg) = pause_messages.pop() {
                            _ = self.self_aid.send(pause_msg);
                        }
                    } else {
                        pause_messages.push(msg);
                    }
                }
            }

            while self.waiting {
                if let Ok(msg) = mailbox.recv() {
                    self.msg_handler(msg);
                    
                    if self.paused {
                        continue 'outer;
                    }
                    
                    if !self.alive {
                        break 'outer;
                    }
                }
            }
            while let Ok(msg) = mailbox.try_recv() {
                self.msg_handler(msg);

                if self.paused {
                    continue 'outer;
                }

                if !self.alive {
                    break 'outer;
                }
            }

            //process task
            let task = self.core.process_task();
            match task {
                SubTask::Idle => {
                    thread::sleep(IDLE_TIME);
                }
                SubTask::Move(pos) => {
                    let _ = self
                        .world_aid
                        .send(WorldManagerMessage::Move(pos, self.self_aid.clone()));
                    self.waiting = true;
                }
                SubTask::Done => {
                    let _ = self
                        .task_aid
                        .send(TaskManagerMessage::GiveMeNewTask(self.self_aid.clone()));
                    self.waiting = true;
                }
                SubTask::GiveItem(to, item) => {
                    self.pending_inventory_task = Some((true, item));
                    let _ = to.send(EntityMessage::GetInventory(self.self_aid.clone()));
                    self.waiting = true;
                    thread::sleep(TRANSFER_TIME);
                }
                SubTask::TakeItem(from, item) => {
                    self.pending_inventory_task = Some((false, item));
                    let _ = from.send(EntityMessage::GetInventory(self.self_aid.clone()));
                    self.waiting = true;
                    thread::sleep(TRANSFER_TIME);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_task() {
        let start_pos = (1, 1);
        let mut core = WorkerCore::new(start_pos);

        let new_pos = (10, 10);
        let task = Task::MoveTo(new_pos);
        core.new_task(task);
        assert_eq!(core.sub_tasks.len(), 1);
        core.process_task();
    }

    #[test]
    fn apply_ok() {
        let start_pos = (1, 1);
        let mut core = WorkerCore::new(start_pos);

        let new_pos = (20, 20);
        let task = Task::MoveTo(new_pos);
        core.new_task(task);
        core.process_task();
        core.apply_ok(new_pos);
        assert_ne!(core.current_pos, start_pos);
    }

    #[test]
    fn apply_err() {
        let start_pos = (1, 1);
        let mut core = WorkerCore::new(start_pos);

        let new_pos = (3, 8);
        let task = Task::MoveTo(new_pos);
        core.new_task(task);
        core.process_task();
        core.apply_err(new_pos);
        assert_eq!(core.current_pos, start_pos);
    }
}
