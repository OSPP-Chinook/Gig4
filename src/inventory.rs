use std::sync::mpsc::{Receiver};

struct Inventory {
    items: usize, // I'm thinking a list or a hashmap or something, array would also work since the inventory size should probably be fixed
}

pub mod inventory {
    use std::sync::mpsc;
    use crate::aid::{self, AID};

    use crate::inventory::Inventory;

    pub fn init<T>(AID: AID<T>, receiver: mpsc::Receiver<T>) { // Not completely sure how the invariant is supposed to be used.
        let inventory: Inventory = Inventory { items: 0 };
        main_loop(inventory);
    }

    fn main_loop(inventory: Inventory) {
        loop {

        }
    }
}