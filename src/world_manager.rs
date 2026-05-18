use std::{
    collections::HashMap,
    sync::{Arc, Mutex, mpsc::Receiver},
    vec,
};

use crate::{
    aid::{AID, AIDHandle},
    building::Building,
    messages::{EntityMessage, MoveError},
    task_manager::{Task, TaskManagerMessage},
    worker::Worker,
    zombie,
};

pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 160;

pub type Pos = (usize, usize);

#[derive(Clone)]
pub enum WorldManagerMessage {
    Quit,
    Move(Pos, AID<EntityMessage>),
    SpawnObstacle(Pos),
    SpawnWorker(Pos),
    SpawnBuilding(Pos, bool),
    KillEntity(AID<EntityMessage>),
}

#[derive(Clone)]
pub enum Tile {
    Empty,
    Obstacle,
    Worker(AID<EntityMessage>),
    Building(AID<EntityMessage>),
}

type WorldLookup = HashMap<AID<EntityMessage>, Pos>;
pub type RawWorldArray = Vec<Vec<Tile>>;
pub type WorldGrid = Arc<Mutex<RawWorldArray>>;

pub fn init_world_grid() -> WorldGrid {
    return Arc::new(Mutex::new(vec![vec![Tile::Empty; WIDTH]; HEIGHT]));
}

fn get_tile(grid: &mut RawWorldArray, pos: Pos) -> Option<&mut Tile> {
    return grid.get_mut(pos.1)?.get_mut(pos.0);
}

fn move_entity(
    grid: &WorldGrid,
    entity_lookup: &mut WorldLookup,
    pos: Pos,
    aid: AID<EntityMessage>,
) {
    let grid = &mut grid.lock().unwrap();

    // check if pos is valid
    if let Some(dest) = get_tile(grid, pos)
        && let Tile::Empty = *dest
        && let Some(old_pos) = entity_lookup.get(&aid)
    {
        let _ = aid.send(EntityMessage::MoveResponse(Ok(pos)));

        // all positions in entity_lookup are valid so unwrap will never panic
        let old_tile = get_tile(grid, *old_pos).unwrap();
        let temp = old_tile.clone();
        *old_tile = Tile::Empty;

        // already checked that pos is valid so unwrap will never panic
        *get_tile(grid, pos).unwrap() = temp;
        entity_lookup.insert(aid, pos);
    } else {
        let _ = aid.send(EntityMessage::MoveResponse(Err(MoveError::Occupied(pos))));
    }
}

fn main(
    this: AID<WorldManagerMessage>,
    mailbox: &Receiver<WorldManagerMessage>,
    task: AID<TaskManagerMessage>,
    grid: WorldGrid,
) {
    let mut entity_lookup: WorldLookup = HashMap::new();

    for msg in mailbox {
        match msg {
            WorldManagerMessage::Quit => break,
            WorldManagerMessage::Move(pos, aid) => move_entity(&grid, &mut entity_lookup, pos, aid),
            WorldManagerMessage::SpawnObstacle(pos) => {
                let grid = &mut grid.lock().unwrap();

                if let Some(dest) = get_tile(grid, pos)
                    && let Tile::Empty = *dest
                {
                    *dest = Tile::Obstacle;
                }
            }
            WorldManagerMessage::SpawnWorker(pos) => {
                let grid = &mut grid.lock().unwrap();

                if let Some(dest) = get_tile(grid, pos)
                    && let Tile::Empty = *dest
                {
                    let aid = Worker::new(this.clone(), task.clone(), pos);
                    *dest = Tile::Worker(aid.clone());
                    entity_lookup.insert(aid, pos);
                }
            }
            WorldManagerMessage::SpawnBuilding(pos, assign_task) => {
                let grid = &mut grid.lock().unwrap();

                if let Some(dest) = get_tile(grid, pos)
                    && let Tile::Empty = *dest
                {
                    let aid = Building::new(this.clone(), task.clone());
                    // temporary until buildings can get tasks some other way
                    if assign_task {
                        let _ = aid.send(EntityMessage::TaskResponse(Ok(Task::Produce(0))));
                    }
                    *dest = Tile::Building(aid.clone());
                    entity_lookup.insert(aid, pos);
                }
            }
            WorldManagerMessage::KillEntity(aid) => {
                let grid = &mut grid.lock().unwrap();

                if let Some(pos) = entity_lookup.remove(&aid) {
                    *get_tile(grid, pos).unwrap() = Tile::Empty;

                    let _ = aid.send(EntityMessage::KillYourself);
                }
            }
        }
    }

    for (entity, _) in entity_lookup {
        let _ = entity.send(EntityMessage::KillYourself);
    }
}

pub fn new_joinable(
    grid: WorldGrid,
    task: AID<TaskManagerMessage>,
) -> (AID<WorldManagerMessage>, AIDHandle) {
    return AID::new_joinable(|aid, mailbox| {
        main(aid, &mailbox, task, grid);

        zombie::world_manager_zombie(mailbox);
    });
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use crate::messages::GetInventoryError;

    use super::*;

    #[test]
    fn kill_entity_on_message() {
        let task = AID::mock().0;
        let grid = init_world_grid();
        let world = new_joinable(grid.clone(), task).0;

        let pos = (0, 0);
        let _ = world.send(WorldManagerMessage::SpawnWorker(pos));
        thread::sleep(Duration::from_millis(250));
        let worker = match get_tile(&mut grid.lock().unwrap(), pos).unwrap() {
            Tile::Worker(aid) => aid.clone(),
            _ => panic!("failed to create worker"),
        };

        let _ = world.send(WorldManagerMessage::KillEntity(worker.clone()));
        thread::sleep(Duration::from_millis(250));
        let (mock, mailbox) = AID::mock();
        assert!(worker.send(EntityMessage::GetInventory(mock)).is_ok());
        assert!(matches!(
            mailbox.recv(),
            Ok(EntityMessage::GetInventoryResponse(Err(
                GetInventoryError::ImDead
            )))
        ));
    }

    #[test]
    fn kill_entity_on_quit() {
        let task = AID::mock().0;
        let grid = init_world_grid();
        let world = new_joinable(grid.clone(), task).0;

        let pos = (0, 0);
        let _ = world.send(WorldManagerMessage::SpawnWorker(pos));
        thread::sleep(Duration::from_millis(250));
        let worker = match get_tile(&mut grid.lock().unwrap(), pos).unwrap() {
            Tile::Worker(aid) => aid.clone(),
            _ => panic!("failed to create worker"),
        };

        let _ = world.send(WorldManagerMessage::Quit);
        thread::sleep(Duration::from_millis(250));
        let (mock, mailbox) = AID::mock();
        assert!(worker.send(EntityMessage::GetInventory(mock)).is_ok());
        assert!(matches!(
            mailbox.recv(),
            Ok(EntityMessage::GetInventoryResponse(Err(
                GetInventoryError::ImDead
            )))
        ));
    }
}
