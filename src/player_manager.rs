use ratatui::macros::vertical;
use ratatui::style::Stylize;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap, Widget};
use ratatui::layout::Constraint::Length;
use ratatui::Frame;
use ratatui::crossterm::event;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::buffer::Buffer;


// Temporary values for world size and stuff while integration isn't working
const WIDTH: usize = 20;
const HEIGHT: usize = 10;
const TILE_SIZE: usize = 2;

pub fn render_loop() -> Result<(), Box<dyn std::error::Error>> {
    ratatui::run(|terminal| loop {
        terminal.draw(render)?;
        if event::read()?.is_key_press() {
            break Ok(());
        }
    })
}

fn render(frame: &mut Frame) {
    let world_array: [[u32; WIDTH]; HEIGHT] = [[0; WIDTH]; HEIGHT];

    let world_area = frame.area().centered(
                Length((WIDTH * 2 + 2) as u16),
                Length((HEIGHT * 2 + 2) as u16)
            );

    let grid = world_area.inner(Margin::new(1, 1));

    frame.render_widget(Block::new().borders(Borders::ALL).title("World"), world_area);

    let row_constraints = (0..HEIGHT).map(|_| Constraint::Length(TILE_SIZE as u16));
    let col_constraints = (0..WIDTH).map(|_| Constraint::Length(TILE_SIZE as u16));
    let horizontal = Layout::horizontal(col_constraints);
    let vertical = Layout::vertical(row_constraints);

    let cells = grid.layout_vec(&vertical).into_iter().flat_map(|row| row.layout_vec(&horizontal));

    for (i, cell) in cells.enumerate() {
        let square = Paragraph::new("╔╗\n╚╝").wrap(Wrap{trim:true});
        frame.render_widget(square, cell);
    }
}