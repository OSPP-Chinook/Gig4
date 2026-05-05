use crate::aid::AID;
use crate::inventory::{self, InventoryMessage};
use crate::item::Item;
use crate::messages::EntityMessage;
use crate::task_manager::{Task, TaskManagerMessage};
use crate::world_manager::{Pos, WorldManagerMessage};
use std::cmp::{max, min};
use std::collections::{HashSet, VecDeque};
use std::sync::mpsc::{Receiver, sync_channel};
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
    pathfinding_state: PathfindingState,
    next_pathfinding_state: Option<PathfindingState>,
    visited_states: HashSet<(Pos, PathfindingState)>,
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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Dir {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum PathfindingState {
    ProductivePath,
    HandRule(usize, Dir),
}

const DEFAULT_PATHFINDING_STATE: PathfindingState = PathfindingState::ProductivePath;

fn abs_sub(a: usize, b: usize) -> usize {
    return max(a, b) - min(a, b);
}

// dx + dy
fn manhattan_distance(from: Pos, to: Pos) -> usize {
    return abs_sub(from.0, to.0) + abs_sub(from.1, to.1);
}

// approximate the direction between to points
fn dir_towards_pos(from: Pos, to: Pos) -> Option<Dir> {
    return if abs_sub(from.0, to.0) >= abs_sub(from.1, to.1) {
        if from.0 < to.0 {
            Some(Dir::Right)
        } else if from.0 > to.0 {
            Some(Dir::Left)
        } else {
            None
        }
    } else {
        if from.1 < to.1 {
            Some(Dir::Down)
        } else if from.1 > to.1 {
            Some(Dir::Up)
        } else {
            None
        }
    };
}

// The counter clockwise angle between two directions in quarter turns
fn dir_angle(from: Dir, to: Dir) -> usize {
    let from = match from {
        Dir::Up => 0,
        Dir::Left => 1,
        Dir::Down => 2,
        Dir::Right => 3,
    };
    let to = match to {
        Dir::Up => 0,
        Dir::Left => 1,
        Dir::Down => 2,
        Dir::Right => 3,
    };
    return (4 + to - from) % 4;
}

// Rotate direction counter clockwise by angle in quarter turns
fn rotate_dir(dir: Dir, angle: usize) -> Dir {
    let dir = match dir {
        Dir::Up => 0,
        Dir::Left => 1,
        Dir::Down => 2,
        Dir::Right => 3,
    };
    match (dir + angle) % 4 {
        0 => Dir::Up,
        1 => Dir::Left,
        2 => Dir::Down,
        3 => Dir::Right,
        _ => unreachable!(),
    }
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
            pathfinding_state: DEFAULT_PATHFINDING_STATE,
            next_pathfinding_state: None,
            visited_states: HashSet::new(),
        }
    }

    fn pathfind(&mut self, dst: Pos) -> Option<(Pos, PathfindingState)> {
        // algorithm from https://en.wikipedia.org/wiki/Maze-solving_algorithm#Maze-routing_algorithm

        let cur = self.current_pos;
        let md_cur = manhattan_distance(cur, dst);

        if cur == dst {
            return None;
        }

        // preferred_dir is direction towards dst if in ProductivePath state
        // or to the right of facing_dir if in HandRule state
        let (md_best, preferred_dir) = match self.pathfinding_state {
            // unwrap will never panic since cur != dst
            PathfindingState::ProductivePath => (md_cur, dir_towards_pos(cur, dst).unwrap()),
            PathfindingState::HandRule(md_best, facing_dir) => (md_best, rotate_dir(facing_dir, 3)),
        };

        let mut path: Option<Pos> = None;

        // try taking productive path if at closest distance
        if md_cur <= md_best {
            for neighbor in self.open_neighbors.iter() {
                if manhattan_distance(*neighbor, dst) < md_cur
                    && (path.is_none()
                        // prioritize moving in the dimension with largest difference
                        || dir_towards_pos(cur, *neighbor) == dir_towards_pos(cur, dst))
                {
                    path = Some(*neighbor);
                }
            }
        }

        if let Some(path_pos) = path {
            // productive path found, enter ProductivePath state
            return Some((path_pos, PathfindingState::ProductivePath));
        }
        // no productive path exists

        // (next pos, dir to next pos, angle between preferred_dir and dir to next pos)
        let mut path: Option<(Pos, Dir, usize)> = None;

        for neighbor in self.open_neighbors.iter() {
            // get direction to neighbor and its angle to preferred_dir
            // unwrap will never panic since cur != *nieghbor
            let neighbor_dir = dir_towards_pos(cur, *neighbor).unwrap();
            let neighbor_angle = dir_angle(preferred_dir, neighbor_dir);
            if let Some((_, _, path_angle)) = path {
                // Take the first path ccw of preferred_dir (including preferred_dir itself)
                if neighbor_angle < path_angle {
                    path = Some((*neighbor, neighbor_dir, neighbor_angle));
                }
            } else {
                // If no other path found yet, take this one
                path = Some((*neighbor, neighbor_dir, neighbor_angle));
            }
        }

        if let Some((path_pos, path_dir, _)) = path {
            // no productive path found, enter HandRule state
            return Some((
                path_pos,
                PathfindingState::HandRule(min(md_cur, md_best), path_dir),
            ));
        } else {
            // completely stuck
            return None;
        }
    }

    fn process_task(&mut self) -> Option<Request> {
        if let Some(sub_task) = self.sub_tasks.front() {
            match sub_task {
                SubTask::Move(pos) => {
                    if *pos == self.current_pos {
                        self.sub_tasks.pop_front();
                        return None;
                    }

                    if let Some((target, state)) = self.pathfind(*pos) {
                        self.pending_move = Some(target);
                        self.next_pathfinding_state = Some(state);
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

            // move successful, update state
            if let Some(state) = self.next_pathfinding_state.take() {
                // since walls can move, the algorithm can get stuck in a loop, if that happens, restart the algorithm by clearing its memory
                if !self.visited_states.insert((pos, state)) {
                    self.visited_states.clear();
                    self.pathfinding_state = DEFAULT_PATHFINDING_STATE;
                } else {
                    self.pathfinding_state = state;
                }
            }
        }
    }
    /// Anropas när WorldManager nekar en flytt.
    /// Tömmer pending_move utan att ändra current_pos.
    fn apply_err(&mut self) {
        if let Some(pos) = self.pending_move.take() {
            // pos is not open
            self.open_neighbors.remove(&pos);
        }

        // move unsuccessful, next state is invalid
        self.next_pathfinding_state = None;
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

    #[test]
    fn dir_towards_pos_test() {
        assert_eq!(dir_towards_pos((0, 0), (0, 0)), None);
        assert_eq!(dir_towards_pos((1, 1), (1, 1)), None);
        assert_eq!(dir_towards_pos((3, 5), (3, 5)), None);

        assert_eq!(dir_towards_pos((0, 1), (0, 0)), Some(Dir::Up));
        assert_eq!(dir_towards_pos((1, 0), (0, 0)), Some(Dir::Left));
        assert_eq!(dir_towards_pos((0, 0), (0, 1)), Some(Dir::Down));
        assert_eq!(dir_towards_pos((0, 0), (1, 0)), Some(Dir::Right));

        assert_eq!(dir_towards_pos((3, 5), (2, 3)), Some(Dir::Up));
        assert_eq!(dir_towards_pos((3, 5), (1, 6)), Some(Dir::Left));
        assert_eq!(dir_towards_pos((3, 5), (5, 4)), Some(Dir::Right));
        assert_eq!(dir_towards_pos((3, 5), (4, 7)), Some(Dir::Down));
    }

    #[test]
    fn dir_angle_test() {
        assert_eq!(dir_angle(Dir::Up, Dir::Up), 0);
        assert_eq!(dir_angle(Dir::Up, Dir::Left), 1);
        assert_eq!(dir_angle(Dir::Up, Dir::Down), 2);
        assert_eq!(dir_angle(Dir::Up, Dir::Right), 3);

        assert_eq!(dir_angle(Dir::Left, Dir::Up), 3);
        assert_eq!(dir_angle(Dir::Left, Dir::Left), 0);
        assert_eq!(dir_angle(Dir::Left, Dir::Down), 1);
        assert_eq!(dir_angle(Dir::Left, Dir::Right), 2);

        assert_eq!(dir_angle(Dir::Down, Dir::Up), 2);
        assert_eq!(dir_angle(Dir::Down, Dir::Left), 3);
        assert_eq!(dir_angle(Dir::Down, Dir::Down), 0);
        assert_eq!(dir_angle(Dir::Down, Dir::Right), 1);

        assert_eq!(dir_angle(Dir::Right, Dir::Up), 1);
        assert_eq!(dir_angle(Dir::Right, Dir::Left), 2);
        assert_eq!(dir_angle(Dir::Right, Dir::Down), 3);
        assert_eq!(dir_angle(Dir::Right, Dir::Right), 0);
    }

    #[test]
    fn rotate_dir_test() {
        assert_eq!(rotate_dir(Dir::Up, 0), Dir::Up);
        assert_eq!(rotate_dir(Dir::Up, 1), Dir::Left);
        assert_eq!(rotate_dir(Dir::Up, 2), Dir::Down);
        assert_eq!(rotate_dir(Dir::Up, 3), Dir::Right);
        assert_eq!(rotate_dir(Dir::Up, 4), Dir::Up);

        assert_eq!(rotate_dir(Dir::Left, 0), Dir::Left);
        assert_eq!(rotate_dir(Dir::Left, 1), Dir::Down);
        assert_eq!(rotate_dir(Dir::Left, 2), Dir::Right);
        assert_eq!(rotate_dir(Dir::Left, 3), Dir::Up);
        assert_eq!(rotate_dir(Dir::Left, 4), Dir::Left);

        assert_eq!(rotate_dir(Dir::Down, 0), Dir::Down);
        assert_eq!(rotate_dir(Dir::Down, 1), Dir::Right);
        assert_eq!(rotate_dir(Dir::Down, 2), Dir::Up);
        assert_eq!(rotate_dir(Dir::Down, 3), Dir::Left);
        assert_eq!(rotate_dir(Dir::Down, 4), Dir::Down);

        assert_eq!(rotate_dir(Dir::Right, 0), Dir::Right);
        assert_eq!(rotate_dir(Dir::Right, 1), Dir::Up);
        assert_eq!(rotate_dir(Dir::Right, 2), Dir::Left);
        assert_eq!(rotate_dir(Dir::Right, 3), Dir::Down);
        assert_eq!(rotate_dir(Dir::Right, 4), Dir::Right);
    }
}
