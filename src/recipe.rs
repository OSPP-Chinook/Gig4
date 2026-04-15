use std::time::Duration;

enum Item {} //Temporary

type ItemQuantity = (Item, usize);

//
pub struct Recipe {
    input: Vec<ItemQuantity>,
    output: Vec<ItemQuantity>,
    pub recipe_time: usize, //recipe time in machine "cycles"/ticks
}