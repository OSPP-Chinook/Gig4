use crate::aid::AID;

mod inventory;
mod aid;

use std::{
    sync::{Arc, Mutex, MutexGuard},
};

fn main() {
    println!("Hello, world!");
}

fn test_inventory() {
    let mut inventories: Vec<Arc<Mutex<(String, usize)>>> = Vec::new();

    for i in 1..4 {
        inventories.push(inventory::inventory::init(format!("Thread {}", i)).1); 
    }

    let mut inv_1: MutexGuard<'_, (String, usize)> = inventories[0].lock().unwrap();

    (*inv_1).1 = 0;

    loop {} // loop forever so the program doesn't terminate and kill the threads
}