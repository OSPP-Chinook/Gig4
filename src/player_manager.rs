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

pub fn render_loop(
    aid: AID<PlayerManagerMessage>,
    mailbox: Receiver<PlayerManagerMessage>,
    world: AID<WorldManagerMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    ratatui::run(|terminal| {
        loop {
            let _ = world.send(WorldManagerMessage::GetDisplay(aid.clone()));

            let mut world_array: WorldGrid =
                std::array::from_fn(|_| std::array::from_fn(|_| Tile::Empty));

            for msg in &mailbox {
                match msg {
                    PlayerManagerMessage::WorldUpdate(arr) => {
                        world_array = arr;
                        let _ = world.send(WorldManagerMessage::GetDisplay(aid.clone()));
                        break;
                    }
                    // TODO: Handle more message types
                    _ => {}
                }
            }

            terminal.draw(|frame| render(frame, world_array))?;

            if poll(Duration::from_secs(0))? {
                match read()? {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        match key_event.code {
                            KeyCode::Char('q') => {
                                break Ok(());
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

fn render(frame: &mut Frame, world_array: WorldGrid) {
    let world_area = frame.area().centered(
        Length((WIDTH * 2 + 2) as u16),
        Length((HEIGHT * 2 + 2) as u16),
    );

    let grid = world_area.inner(Margin::new(1, 1));

    frame.render_widget(
        Block::new().borders(Borders::ALL).title("World"),
        world_area,
    );

    // Vectors av HEIGHT eller WIDTH stycken constraints, alla lika stora som TILE_SIZE
    let row_constraints = (0..HEIGHT).map(|_| Constraint::Length(TILE_SIZE as u16));
    let col_constraints = (0..WIDTH).map(|_| Constraint::Length(TILE_SIZE as u16));

    // Split up horizontal and vertical layouts after the rows and columns
    let horizontal = Layout::horizontal(col_constraints);
    let vertical = Layout::vertical(row_constraints);

    // Gör en 2d array av cells
    let rows: Vec<Rect> = grid.layout_vec(&vertical);
    let grid_array: Vec<Vec<Rect>> = rows
        .iter()
        .map(|row: &Rect| row.layout_vec(&horizontal))
        .collect();

    // TODO: Det här är lätt att förstå men kan vara RUSTigare
    for (row, y) in grid_array.iter().zip(0..HEIGHT) {
        for (cell, x) in row.iter().zip(0..WIDTH) {
            let tile = &world_array[y][x];
            // check if tile is empty
            match tile {
                Tile::Empty => {}
                Tile::Worker(_) => {
                    let square = Paragraph::new("╭╮\n╰╯").blue();
                    frame.render_widget(square, *cell);
                }
                Tile::Building(_) => {
                    let square = Paragraph::new("╔╗\n╚╝").red();
                    frame.render_widget(square, *cell);
                }
            }
        }
    }
}
