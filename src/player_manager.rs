use ratatui::macros::vertical;
use ratatui::style::Stylize;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap, Widget};
use ratatui::layout::Constraint::Length;
use ratatui::Frame;
use ratatui::crossterm::event;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::buffer::Buffer;
use crossterm::event::{KeyCode, KeyEventKind};


// Temporary values for world size and stuff while integration isn't working
const WIDTH: usize = 20;
const HEIGHT: usize = 10;
const TILE_SIZE: usize = 2;

pub fn render_loop() -> Result<(), Box<dyn std::error::Error>> {
    ratatui::run(|terminal| loop {
        terminal.draw(render)?;

        if let Some(key) = event::read()?.as_key_press_event() {
            match key.code {
                KeyCode::Char('q') => {
                    break Ok(());
                }
                _ => {}
            }
        }
    })
}

fn render(frame: &mut Frame) {
    let mut world_array: [[u32; WIDTH]; HEIGHT] = [[0; WIDTH]; HEIGHT];
    world_array[2][2] = 1;

    let world_area = frame.area().centered(
                Length((WIDTH * 2 + 2) as u16),
                Length((HEIGHT * 2 + 2) as u16)
            );

    let grid = world_area.inner(Margin::new(1, 1));

    frame.render_widget(Block::new().borders(Borders::ALL).title("World"), world_area);

    // Vectors av HEIGHT eller WIDTH stycken constraints, alla lika stora som TILE_SIZE
    let row_constraints = (0..HEIGHT).map(|_| Constraint::Length(TILE_SIZE as u16));
    let col_constraints = (0..WIDTH).map(|_| Constraint::Length(TILE_SIZE as u16));

    // Split up horizontal and vertical layouts after the rows and columns
    let horizontal = Layout::horizontal(col_constraints);
    let vertical = Layout::vertical(row_constraints);

    // Gör en 2d array av cells
    let rows: Vec<Rect> = grid.layout_vec(&vertical);
    let grid_array: Vec<Vec<Rect>> = rows.iter().map(|row: &Rect| row.layout_vec(&horizontal)).collect();

    // TODO: Det här är lätt att förstå men kan vara RUSTigare
    for (row, y) in grid_array.iter().zip(0..HEIGHT) {
        for (cell, x) in row.iter().zip(0..WIDTH) {
            // TODO: world_array kommer ersättas med messages till world manager?
            if world_array[y][x] == 1 {
                let square = Paragraph::new("╔╗\n╚╝").red();
                frame.render_widget(square, *cell);
            }
        }
    }
}