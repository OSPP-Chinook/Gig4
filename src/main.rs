mod aid;
mod player_manager;

fn main() {
    println!("Hello, world!");
    let _ = player_manager::render_loop();
}
