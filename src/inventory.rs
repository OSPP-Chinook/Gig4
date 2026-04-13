use std::sync::mpsc::{Receiver};

struct Inventory {
    items: usize,
}

pub mod inventory {
    use std::sync::mpsc;
    use crate::aid::{self, AID};

    use crate::inventory::Inventory;

    pub fn init<T>(AID: AID<T>, receiver: mpsc::Receiver<T>) {
        let inventory: Inventory = Inventory { items: 0 };
        main_loop(inventory);
    }

    fn main_loop(inventory: Inventory) {
        loop {

        }
    }
}