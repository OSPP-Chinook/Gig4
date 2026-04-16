use crate::{
    aid::AID,
    messages::{PlayerManagerMessage, TaskManagerMessage, WorldManagerMessage},
};

pub struct GameManager {
    world: AID<WorldManagerMessage>,
    task: AID<TaskManagerMessage>,
    player: AID<PlayerManagerMessage>,
}

impl GameManager {
    pub fn new() -> Self {
        let world = AID::new(|_, _| {});
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
        todo!("Demo not yet implemented");
    }
}
