use crate::{
    aid::AID,
    messages::{PlayerManagerMessage, TaskManagerMessage},
    world_manager::{self, WorldManagerMessage},
    building::Building,
};

pub struct GameManager {
    world: AID<WorldManagerMessage>,
    task: AID<TaskManagerMessage>,
    player: AID<PlayerManagerMessage>,
}

impl GameManager {
    pub fn new() -> Self {
        let world = AID::new(world_manager::main);
        let task = AID::new(|_, _| {});
        let player = AID::new(|_, _| {});

        Self { world, task, player }
    }

    pub fn run(&self) {
        self.demo();

        loop {
            std::thread::park();
        }
    }

    fn demo(&self) {
        let building = Building::new(self.world.clone());
        let _ = self.world.send(WorldManagerMessage::Move((0, 0), building));
    }
}
