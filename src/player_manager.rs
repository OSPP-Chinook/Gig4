use std::sync::mpsc::Receiver;
use std::time::Duration;

use crate::{
    aid::AID,
    messages::PlayerManagerMessage,
    world_manager::{HEIGHT, Tile, WIDTH, WorldGrid, WorldManagerMessage},
};
use crossterm::event::{Event, KeyCode, KeyEventKind, poll, read};
use ratatui::Frame;
use ratatui::layout::Constraint::Length;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::Stylize;
use ratatui::widgets::{Block, Borders, Paragraph};

// Width and height of a tile on the screen in characters
const TILE_SIZE: usize = 2;

#[derive(Copy, Clone)]
struct Camera(i32, i32);

impl Camera {
    fn change(&mut self, dx: i32, dy: i32) {
        // limit camera from going outside world
        let width = WIDTH.try_into().unwrap();
        let height = HEIGHT.try_into().unwrap();
        let mut x = self.0 + dx;
        let mut y = self.1 + dy;
        if x < 0 {
            x = 0;
        }
        if y < 0 {
            y = 0;
        }
        if x >= width {
            x = width - 1;
        }
        if y >= height {
            y = height - 1;
        }

        self.0 = x;
        self.1 = y;
    }
}

// Default: 1. Set to -1 for inverted movement.
// The default setting looks weird now, but it will make sense when the world is more populated.
const MOVE_CAMERA: i32 = 1;

pub fn render_loop(
    aid: AID<PlayerManagerMessage>,
    mailbox: Receiver<PlayerManagerMessage>,
    world: AID<WorldManagerMessage>,
    world_array: WorldGrid,
) -> Result<(), Box<dyn std::error::Error>> {
    ratatui::run(|terminal| {
        // camera starts centered on the world
        let mut camera = Camera(
            (WIDTH / 2).try_into().unwrap(),
            (HEIGHT / 2).try_into().unwrap(),
        );

        loop {
            //read all messages in mailbox
            while let Ok(msg) = mailbox.try_recv() {
                match msg {
                    // TODO: Handle more message types
                    _ => {}
                }
            }

            terminal.draw(|frame| render(frame, &world_array, camera))?;

            if poll(Duration::from_millis(30))? {
                match read()? {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        match key_event.code {
                            KeyCode::Char('q') => {
                                break Ok(());
                            }
                            KeyCode::Char('w') => {
                                camera.change(0, -MOVE_CAMERA);
                            }
                            KeyCode::Char('s') => {
                                camera.change(0, MOVE_CAMERA);
                            }
                            KeyCode::Char('a') => {
                                camera.change(-MOVE_CAMERA, 0);
                            }
                            KeyCode::Char('d') => {
                                camera.change(MOVE_CAMERA, 0);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    })
}

fn render(frame: &mut Frame, world_array: &WorldGrid, camera: Camera) {
    let world_area = frame.area().inner(Margin::new(4, 4));

    frame.render_widget(
        Block::new().borders(Borders::ALL).title("World"),
        world_area.outer(Margin::new(1, 1)),
    );

    let box_w = world_area.width / 2;
    let box_h = world_area.height / 2;

    for y in 0..box_h {
        for x in 0..box_w {
            // draw a background in the "World" area
            // this is so the player can tell the difference between buildable area and surrounding borders
            let rect = Rect::new(world_area.x + 2 * x, world_area.y + 2 * y, 2, 2);
            let square = Paragraph::new(".").gray();
            frame.render_widget(square, rect);
        }
    }

    // aquire lock until it falls out of scope
    let world_array = &world_array.lock().unwrap();

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let tile = &world_array[y][x];

            let y: i32 = y.try_into().unwrap();
            let x: i32 = x.try_into().unwrap();
            let draw_pos = (
                x + (box_w / 2) as i32 - camera.0,
                y + (box_h / 2) as i32 - camera.1,
            );
            let rect_at_pos = if 0 <= draw_pos.0
                && draw_pos.0 < box_w.into()
                && 0 <= draw_pos.1
                && draw_pos.1 < box_h.into()
            {
                // tile in visible grid
                Rect::new(
                    world_area.x + (2 * draw_pos.0 as u16),
                    world_area.y + (2 * draw_pos.1 as u16),
                    2,
                    2,
                )
            } else {
                continue;
            };

            match tile {
                Tile::Empty => {
                    // overwrite background
                    let square = Paragraph::new("  \n  ");
                    frame.render_widget(square, rect_at_pos);
                }
                Tile::Obstacle => {
                    let square = Paragraph::new("██\n██").green();
                    frame.render_widget(square, rect_at_pos);
                }
                Tile::Worker(_aid) => {
                    let square = Paragraph::new("╭╮\n╰╯").blue();
                    frame.render_widget(square, rect_at_pos);
                }
                Tile::Building(_aid) => {
                    let square = Paragraph::new("╔╗\n╚╝").red();
                    frame.render_widget(square, rect_at_pos);
                }
            }
        }
    }
}
