use std::{
    io::{self},
    thread,
    time::Duration,
};
use termion::{event::Key, input::TermRead, raw::IntoRawMode};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};

struct Task {
    description: String,
}

impl Task {
    fn new(description: String) -> Task {
        Task { description }
    }
}

struct AppState {
    tasks: Vec<Task>,
    input: String,
    input_mode: bool,
    edit_mode: bool,
    selected_task: Option<usize>,
}

impl AppState {
    fn new() -> AppState {
        AppState {
            tasks: Vec::new(),
            input: String::new(),
            input_mode: false,
            edit_mode: false,
            selected_task: Some(0),
        }
    }

    fn add_task(&mut self, description: String) {
        let task = Task::new(description);
        self.tasks.push(task);
    }

    fn delete_task(&mut self) {
        if let Some(index) = self.selected_task {
            if index < self.tasks.len() {
                self.tasks.remove(index);
            }
        }
    }

    fn update_task(&mut self, description: String) {
        if let Some(index) = self.selected_task {
            if let Some(task) = self.tasks.get_mut(index) {
                task.description = description;
            }
        }
    }
}

fn main() -> Result<(), io::Error> {
    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app_state = AppState::new();
    let mut keys = io::stdin().keys();

    terminal.draw(|f| {
        let size = f.size();
        let block = Block::default().title("Task List").borders(Borders::ALL);
        f.render_widget(block, size)
    })?;

    loop {
        if let Some(Ok(key)) = keys.next() {
            match key {
                Key::Char('q') => break,
                Key::Char('n') if !app_state.input_mode => {
                    app_state.input_mode = true;
                    app_state.input.clear();
                }
                Key::Char('\n') if app_state.input_mode => {
                    app_state.add_task(app_state.input.clone());
                    app_state.input_mode = false;
                }
                Key::Char(c) if app_state.input_mode => {
                    app_state.input.push(c);
                }
                Key::Backspace if app_state.input_mode => {
                    app_state.input.pop();
                }
                Key::Up => {
                    if let Some(selected) = app_state.selected_task {
                        app_state.selected_task = Some(selected.saturating_sub(1));
                    }
                }
                Key::Down => {
                    if let Some(selected) = app_state.selected_task {
                        app_state.selected_task =
                            Some((selected + 1).min(app_state.tasks.len().saturating_sub(1)));
                    }
                }
                _ => {}
            }
        }

        terminal.draw(|f| {
            let size = f.size();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(size);

            let input_text = if app_state.input_mode {
                format!("Input Mode: {}", app_state.input)
            } else {
                "Press 'n' to add a task".to_string()
            };

            let input_paragraph = Paragraph::new(input_text)
                .block(Block::default().borders(Borders::ALL).title("Input"));
            f.render_widget(input_paragraph, chunks[0]);

            let tasks: Vec<ListItem> = app_state
                .tasks
                .iter()
                .enumerate()
                .map(|(i, task)| {
                    let content = task.description.as_str();
                    let item = ListItem::new(content);
                    if Some(i) == app_state.selected_task {
                        item.style(Style::default().fg(Color::Yellow))
                    } else {
                        item
                    }
                })
                .collect();

            let tasks_list =
                List::new(tasks).block(Block::default().borders(Borders::ALL).title("Tasks"));
            f.render_widget(tasks_list, chunks[1])
        })?;

        thread::sleep(Duration::from_millis(100));
    }

    terminal.clear()?;
    terminal.set_cursor(0, 0)?;
    terminal.show_cursor()?;
    Ok(())
}
