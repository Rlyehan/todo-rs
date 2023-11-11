use std::{io, thread, time::Duration};
use termion::{event::Key, input::TermRead, raw::IntoRawMode};
use tui::{
    backend::TermionBackend,
    widgets::{Block, Borders},
    Terminal,
};

fn main() -> Result<(), io::Error> {
    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    terminal.draw(|f| {
        let size = f.size();
        let block = Block::default().title("Task List").borders(Borders::ALL);
        f.render_widget(block, size)
    })?;

    let mut keys = io::stdin().keys();
    loop {
        if let Some(Ok(key)) = keys.next() {
            match key {
            Key::Char('q') => break,
            _ => {}
            }
        }

        terminal.draw(|f| {
        let size = f.size();
        let block = Block::default().title("Task List").borders(Borders::ALL);
        f.render_widget(block, size)
        })?;

        thread::sleep(Duration::from_millis(100));
    }

    terminal.clear()?;
    terminal.set_cursor(0, 0)?;
    terminal.show_cursor()?;
    Ok(())
}
