use crate::{
    aid::AID, 
};

struct Inventory {
    aid: AID<InventoryMessage>,
    max: usize,
    items: (String, usize), // Resources are stored as (String, usize) where String is name of resource and usize is count
}

impl Inventory {
    fn construct(aid: AID<InventoryMessage>) -> Self {
        return Inventory { aid, max: 10, items: (String::from("Mutexium"), 0) };
    }

    fn is_empty(&self) -> bool {
        return self.items.1 == 0;
    }

    fn is_full(&self) -> bool {
        return self.items.1 == self.max;
    }

    // For debugging as of now
    fn print_inv(&self, name: String) {
        println!("{0}: {1} - {2}/{3}", name, self.items.0, self.items.1, self.max);
    }
}

#[derive(Clone)]
pub enum InventoryMessage {
    // The following are sent by owner (entity)
    Increase,
    Decrease,
    TakeFrom(AID<InventoryMessage>), 
    Kill,

    // The following are sent by another inventory
    GiveMeItem(AID<InventoryMessage>),                  // SHOULD ONLY BE CALLED USING TakeFrom MESSAGE
    ReceiveItem(Result<(String, usize), &'static str>), // SHOULD ONLY BE CALLED USING GiveMeItem MESSAGE
}

pub mod inventory {
    use crate::{
        aid::AID, 
        inventory::{Inventory, InventoryMessage}
    };


    /// Initializes a new inventory and returns its AID
    pub fn init() -> AID<InventoryMessage> {
        return AID::new(inventory_loop);
    }

    fn inventory_loop(
            aid: AID<InventoryMessage>, 
            mailbox: std::sync::mpsc::Receiver<InventoryMessage>
        ){
        let mut inventory: Inventory = Inventory::construct(aid);
        let mut transfer_in_process: bool = false; 

        loop {
            match mailbox.recv().unwrap() { // Recv blocks the thread until a message is recieved which seems right for this
                InventoryMessage::Increase => increase(&mut inventory),
                InventoryMessage::Decrease => decrease(&mut inventory),
                InventoryMessage::TakeFrom(other) => 
                    take_from(&inventory, other, &mut transfer_in_process),
                InventoryMessage::Kill => return, 

                InventoryMessage::GiveMeItem(sender) => 
                    give_me_item(&mut inventory, sender, &mut transfer_in_process),
                InventoryMessage::ReceiveItem(result) => {
                    receive_item(&mut inventory, result);
                    transfer_in_process = false;
                }
            };

            inventory.print_inv(String::from("Inventory"));
        }
    }

    /// Increase this inventory by one, will not do anything if inventory is full.
    /// 
    /// # Arguments
    /// 
    /// * 'inventory' - Mutable reference to the inventory to increase
    fn increase(inventory: &mut Inventory) {
        if inventory.is_full() {
            return; // Error or something
        }

        inventory.items.1 += 1;

        return; // Success or something
    }

    /// Decrease this inventory by one, will not do anything if inventory is empty.
    /// 
    /// # Arguments
    /// 
    /// * 'inventory' - Mutable reference to the inventory to decrease
    fn decrease(inventory: &mut Inventory) {
        if inventory.is_empty() {
            return; // Error or something
        }

        inventory.items.1 -= 1;

        return; // Success or something
    }

    /// Asks the other inventory to perform the give_me_item function with 
    /// this inventorys AID as 'sender' parameter
    /// 
    /// # Arguments
    /// 
    /// * 'inventory'            - Reference to the inventory to move item to 
    /// * 'aid'                  - AID of the inventory to take from
    /// * 'transfer_in_progress' - Bool that is true if there is already a transfer in progress, else false
    fn take_from(
        inventory: &Inventory, 
        aid: AID<InventoryMessage>, 
        transfer_in_progress: &mut bool
    ) {
        if *transfer_in_progress || inventory.is_full() {
            return; // Should send an error or whatever
        }

        *transfer_in_progress = true;
        _ = aid.send(InventoryMessage::GiveMeItem(inventory.aid.clone())); // Should handle Result in some way
    }

    /// Checks if this inventory can give item and sends a result containing either a tuple containing
    /// what item and quantity, or an error containing a string explaining what went wrong.
    /// 
    /// # Arguments
    /// 
    /// * 'inventory'            - Mutable reference to this inventory
    /// * 'sender'               - AID of the requesting inventory
    /// * 'transfer_in_progress' - Bool that is true if there is already a transfer in progress, else false
    fn give_me_item(
        inventory: &mut Inventory, 
        sender: AID<InventoryMessage>, 
        transfer_in_progress: &mut bool
    ) {
        if *transfer_in_progress {
            _ = sender.send(InventoryMessage::ReceiveItem(Result::Err("I'm busy!"))); // Sends an error; TODO: Should handle Result in some way
            return;
        }

        if inventory.is_empty() {
            _ = sender.send(InventoryMessage::ReceiveItem(Result::Err("I'm empty!"))); // Sends an error; TODO: Should handle Result in some way
            return;
        }


        inventory.items.1 -= 1;
        _ = sender.send(InventoryMessage::ReceiveItem(Result::Ok((inventory.items.0.clone(), 1)))); // Sends a tuple containing what item it is and how many it moved; TODO: Should handle Result in some way
    }

    /// Gets the result from a GiveMeItem message and add the item to this inventory, or prints the
    /// error message if GiveMeItem failed.
    /// 
    /// # Arguments
    /// 
    /// * 'inventory' - Mutable reference to this inventory
    /// * 'result'    - A result containing either a tuple of what item was moved and the quantity, 
    ///                 or the error message as a str
    fn receive_item(inventory: &mut Inventory, result: Result<(String, usize), &'static str>) {
        match result {
            Ok(item) => { inventory.items.1 += item.1; }, // This is a placeholder
            Err(msg) => { println!("{}", msg) } // should probably do something else
        };
    }
}