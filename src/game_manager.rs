use std::{
    thread,
    sync::Arc,
    time::Duration,
    path::Path,
};

use crate::{
    aid::AID,
    assets::{Assets, AssetError},
    building::Building,
    entity::Entity,
    messages::{PlayerManagerMessage, TaskManagerMessage, Task, EntityMessage},
    player_manager,
    world_manager::{self, WorldManagerMessage},
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
        let world = AID::new(world_manager::main);
        let task = AID::new(|_, _| {});
        let player = AID::new({
            let world = world.clone();
            move |aid, mailbox| {
                let _ = player_manager::render_loop(aid, mailbox, world);
            }
        });

        Ok(Self { assets, world, task, player })
    }

    pub fn run(&self) {
        self.demo();
        loop { thread::park(); }
    }

    fn demo(&self) {
        let world = &self.world;
        
        let worker = Entity::new(world.clone(), self.task.clone(), (10, 3));
        let building1 = Building::new(world.clone(), );
        let building2 = Building::new(world.clone());

        let _ = world.send(WorldManagerMessage::Move((10, 3), worker.clone()));
        let _ = world.send(WorldManagerMessage::Move((3, 3), building1));
        let _ = world.send(WorldManagerMessage::Move((15, 3), building2));

        thread::sleep(Duration::from_secs(1));
        let _ = worker.send(EntityMessage::Task(Task::MoveTo((14, 3))));
        thread::sleep(Duration::from_secs(1));
    }
}
