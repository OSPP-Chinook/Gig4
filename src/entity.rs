use crate::aid::AID;
use crate::inventory::{InventoryMessage, inventory};
use crate::messages::{EntityMessage, Task, TaskManagerMessage};
use crate::world_manager::{Pos, WorldManagerMessage};
use std::sync::mpsc::Receiver;

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
    is_busy: bool,
}

#[allow(dead_code)]
impl EntityCore {
    // skapar en EntityCore  med given start position
    fn new(start_pos: Pos) -> EntityCore {
        EntityCore {
            current_pos: start_pos,
            pending_move: None,
            is_busy: false,
        }
    }

    /// Behandlar en Task och returnerar eventuell Move-position
    /// som Entity-aktorn ska skicka till WorldManager.
    fn apply_task(&mut self, task: Task) -> Option<Pos> {
        match task {
            Task::MoveTo(pos) => {
                self.pending_move = Some(pos);
                self.is_busy = true;
                Some(pos)
            }

            Task::AddItem { .. } => {
                self.is_busy = true;
                None
            }
            Task::RemoveItem { .. } => {
                self.is_busy = true;
                None
            }
            Task::TakeFrom { .. } => {
                self.is_busy = true;
                None
            }
            Task::GiveTo { .. } => {
                self.is_busy = true;
                None
            }
            Task::PrintInventory(_) => {
                self.is_busy = true;
                None
            }

            Task::Idle => None,
        }
    }
    /// Anropas när WorldManager godkänner en flytt.
    /// Uppdaterar current_pos och tömmer pending_move.
    fn apply_ok(&mut self) {
        if let Some(pos) = self.pending_move.take() {
            self.current_pos = pos;
            self.is_busy = false;
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
    world_aid: AID<WorldManagerMessage>,
    task_aid: AID<TaskManagerMessage>,
    inventory: AID<InventoryMessage>,
    mailbox: AID<EntityMessage>,
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
        mailbox: AID<EntityMessage>,
        world: AID<WorldManagerMessage>,
        task: AID<TaskManagerMessage>,
        start_pos: Pos,
    ) -> Self {
        Entity {
            core: EntityCore::new(start_pos),
            world_aid: world,
            task_aid: task,
            inventory: inventory::init(),
            mailbox: mailbox,
        }
    }

    fn run(&mut self, mailbox: Receiver<EntityMessage>) {
        for msg in mailbox {
            match msg {
                EntityMessage::Task(task) => match task {
                    Task::MoveTo(_pos) => {
                        if let Some(pos) = self.core.apply_task(task) {
                            let _ = self
                                .world_aid
                                .send(WorldManagerMessage::Move(pos, self.mailbox.clone()));
                        }
                    }

                    Task::AddItem { item, amount } => {
                        let _ = self.inventory.send(InventoryMessage::Add((item, amount)));
                    }

                    Task::RemoveItem { item, amount } => {
                        let _ = self
                            .inventory
                            .send(InventoryMessage::Remove((item, amount)));
                    }

                    Task::TakeFrom { from, item, amount } => {
                        let _ = self
                            .inventory
                            .send(InventoryMessage::TakeFrom(from, (item, amount)));
                    }

                    Task::GiveTo { to, item, amount } => {
                        let _ = self
                            .inventory
                            .send(InventoryMessage::GiveTo(to, (item, amount)));
                    }

                    Task::PrintInventory(name) => {
                        let _ = self.inventory.send(InventoryMessage::PrintInventory(name));
                    }

                    Task::Idle => {}
                },

                EntityMessage::KillYourself => {
                    let _ = self
                        .world_aid
                        .send(WorldManagerMessage::KillMe(self.mailbox.clone()));
                    break;
                }

                EntityMessage::Ok => {
                    //world manager godkände flyyten
                    //uppdatera EntityCore-> cunnrent_pos
                    self.core.apply_ok();
                    self.core.is_busy = false;
                }

                EntityMessage::Err => {
                    // world manager neckade flytten
                    // ingen ändring i pos
                    self.core.apply_err();
                    self.core.is_busy = false;
                }

                EntityMessage::InventoryOk =>{

                    //tillfälligt lösning
                    self.core.is_busy = false;
                }


                EntityMessage::InventoryErr =>{

                    //tillfälligt lösning
                    self.core.is_busy = false;

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

        assert_eq!(core.apply_task(task), Some(new_pos));
        assert_eq!(core.pending_move, Some(new_pos));
    }

    #[test]
    fn apply_ok() {
        let start_pos = (1, 1);
        let mut core = EntityCore::new(start_pos);

        let new_pos = (20, 20);
        let task = Task::MoveTo(new_pos);
        core.apply_task(task);
        core.apply_ok();
        assert_eq!(core.current_pos, new_pos);
    }

    #[test]
    fn apply_err() {
        let start_pos = (1, 1);
        let mut core = EntityCore::new(start_pos);

        let new_pos = (3, 8);

        let task = Task::MoveTo(new_pos);

        core.apply_task(task);
        core.apply_err();

        assert_eq!(core.current_pos, start_pos);
        assert_eq!(core.pending_move, None);
    }

    #[test]

    fn is_bussy() {
        let start_pos = (10, 10);
        let mut core = EntityCore::new(start_pos);
        assert_eq!(core.is_busy, false);

        let new_pos = (20, 20);
        let task = messages::Task::MoveTo(new_pos);
        core.apply_task(task);

        assert_eq!(core.is_busy, true);
    }
}
