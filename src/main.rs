use crate::aid::AID;

mod inventory;
mod aid;

use core::time;
use std::{
    sync::{Arc, Mutex, MutexGuard}, thread::sleep,
};

fn main() {
    println!("Hello, world!");
    test_inventory2();
}

fn test_inventory() {
    let mut inventories: Vec<Arc<Mutex<(String, usize)>>> = Vec::new();

    for i in 1..4 {
        inventories.push(inventory::inventory::init().1); 
    }

    let mut inv_1: MutexGuard<'_, (String, usize)> = inventories[0].lock().unwrap();

    (*inv_1).1 = 0;

    loop {} // loop forever so the program doesn't terminate and kill the threads
}

fn test_inventory2() {
    let inv_a: Arc<Mutex<(String, usize)>> = inventory::inventory::init().1;
    let inv_b: Arc<Mutex<(String, usize)>> = inventory::inventory::init().1;

    for _i in 0..10 {
        inventory::inventory::increase(&inv_a);
    }

    for _i in 0..10 {
        inventory::inventory::r#move(&inv_a, &inv_b);

        inventory::inventory::print_inv(&inv_a, String::from("Inventory A"));
        inventory::inventory::print_inv(&inv_b, String::from("Inventory B"));

        sleep(time::Duration::from_millis(500));
    }
}