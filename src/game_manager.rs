use std::{path::Path, sync::Arc, thread, time::Duration};

use crate::{
    aid::AID,
    assets::{AssetError, Assets},
    building::Building,
    entity::Entity,
    messages::{EntityMessage, PlayerManagerMessage, Task, TaskManagerMessage},
    player_manager,
    world_manager::{self, WorldManagerMessage, init_world_grid},
};

pub struct GameManager {
    assets: Arc<Assets>,
    world: AID<WorldManagerMessage>,
    task: AID<TaskManagerMessage>,
    player: AID<PlayerManagerMessage>,
}

impl GameManager {
    pub fn new() -> Result<Self, AssetError> {
        let assets = Arc::new(Assets::load(Path::new("assets"))?);

        let grid = init_world_grid();
        let world = AID::new({
            let grid = grid.clone();
            |aid, mailbox| world_manager::main(aid, mailbox, grid)
        });
        let task = AID::new(|_, _| {});
        let player = AID::new({
            let world = world.clone();
            let grid = grid.clone();
            |aid, mailbox| {
                player_manager::render_loop(aid, mailbox, world, grid);
            }
        });

        Ok(Self {
            assets,
            world,
            task,
            player,
        })
    }

    pub fn run(&self) {
        self.demo();
        loop {
            thread::park();
        }
    }

    fn demo(&self) {
        let building = Building::new(self.world.clone());
        let building2 = Building::new(self.world.clone());
        let _ = self
            .world
            .send(WorldManagerMessage::PlaceBuilding((3, 3), building.clone()));
        let _ = self.world.send(WorldManagerMessage::PlaceBuilding(
            (15, 3),
            building2.clone(),
        ));

        let mut x = 10;
        let y = 3;

        let worker = Entity::new(self.world.clone(), self.task.clone(), (10, 3));
        let _ = self
            .world
            .send(WorldManagerMessage::PlaceWorker((x, y), worker.clone()));

        loop {
            while x < 14 {
                thread::sleep(Duration::from_millis(250));
                x += 1;
                let _ = worker.send(crate::messages::EntityMessage::Task(
                    crate::messages::Task::MoveTo((x, y)),
                ));
            }
            thread::sleep(Duration::from_millis(2500));

            while x > 4 {
                thread::sleep(Duration::from_millis(250));
                x -= 1;
                let _ = worker.send(crate::messages::EntityMessage::Task(
                    crate::messages::Task::MoveTo((x, y)),
                ));
            }
            thread::sleep(Duration::from_millis(2500));
        }
    }
}
