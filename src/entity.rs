use crate::aid::AID;
use crate::messages::{EntityMessage, Task, EntityMailbox};
use crate::world_manager::{WorldManagerMessage, Pos};
use crate::task_manager::TaskManagerMessage;

pub struct Entity {
    world: AID<WorldManagerMessage>,
    task: AID<TaskManagerMessage>,
    pos: Pos,
    self_aid: AID<EntityMessage>,
}

impl Entity {
    pub fn new(
        world: AID<WorldManagerMessage>,
        task: AID<TaskManagerMessage>,
        start_pos: Pos,
    ) -> AID<EntityMessage> {
        AID::new(move |aid, mailbox| {
            let mut entity = Entity {
                world,
                task,
                pos: start_pos,
                self_aid: aid.clone(),
            };
            entity.run(mailbox);
        })
    }

    fn run(&mut self, mailbox: EntityMailbox) {
        for (msg, _sender_actor) in mailbox {
            match msg {
                EntityMessage::Task(task) => {
                    self.handle_task(task);
                }
                EntityMessage::KillYourself => {
                    let _ = self.world.send(WorldManagerMessage::KillMe(self.self_aid.clone()));
                    break;
                }
                EntityMessage::Ok => {
                    // world manager godkände flytten
                    // uppdatera position här om du vill
                }
                EntityMessage::Err => {
                    // world manager nekade flytten
                }
            }
        }
    }

    fn handle_task(&mut self, _task: Task) {
        let new_pos = (self.pos.0 + 1, self.pos.1);
        let _ = self.world.send(WorldManagerMessage::Move(new_pos, self.self_aid.clone()));
    }
}
