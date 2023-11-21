use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
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

#[derive(Serialize, Deserialize)]
struct Task {
    description: String,
}

impl Task {
    fn new(description: String) -> Task {
        Task { description }
    }
}

#[derive(PartialEq)]
enum Mode {
    Normal,
    Input,
    Edit,
    DeleteConfirm,
}

struct AppState {
    tasks: Vec<Task>,
    input: String,
    mode: Mode,
    selected_task: Option<usize>,
}

impl AppState {
    fn new() -> AppState {
        AppState {
            tasks: Vec::new(),
            input: String::new(),
            mode: Mode::Normal,
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

    fn load_tasks(&mut self, file_path: &str) {
        match File::open(file_path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                match serde_json::from_reader(reader) {
                    Ok(tasks) => self.tasks = tasks,
                    Err(e) => eprintln!("Failed to parse tasks from JSON: {}", e),
                }
            }
            Err(e) => eprintln!("Failed to open file: {}", e),
        }
    }

    fn save_tasks(&mut self, file_path: &str) {
        let file = File::create(file_path).unwrap();
        serde_json::to_writer(file, &self.tasks).unwrap();
    }
}

fn main() -> Result<(), io::Error> {
    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app_state = AppState::new();
    app_state.load_tasks("tasks.json");
    println!("Loaded {} tasks", app_state.tasks.len());
    let mut keys = io::stdin().keys();

    terminal.draw(|f| {
        let size = f.size();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
            .split(size);

        let (title, input_text) = match app_state.mode {
            Mode::Input => ("Input", format!("Input Mode: {}", app_state.input)),
            Mode::Edit => ("Edit", format!("Editing: {}", app_state.input)),
            _ => ("Input", "Press 'n' to add a task".to_string()),
        };

        let input_paragraph =
            Paragraph::new(input_text).block(Block::default().borders(Borders::ALL).title(title));
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

    loop {
        if let Some(Ok(key)) = keys.next() {
            match app_state.mode {
                Mode::Normal => match key {
                    Key::Char('q') => break,
                    Key::Char('n') => {
                        app_state.mode = Mode::Input;
                        app_state.input.clear();
                    }
                    Key::Char('d') if app_state.selected_task.is_some() => {
                        app_state.mode = Mode::DeleteConfirm;
                    }
                    Key::Char('e') if app_state.selected_task.is_some() => {
                        app_state.mode = Mode::Edit;
                        app_state.input = app_state.tasks[app_state.selected_task.unwrap()]
                            .description
                            .clone();
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
                },
                Mode::DeleteConfirm => match key {
                    Key::Char('d') => {
                        app_state.delete_task();
                        app_state.mode = Mode::Normal;
                    }
                    _ => app_state.mode = Mode::Normal,
                },
                Mode::Input | Mode::Edit => match key {
                    Key::Char('\n') => {
                        if let Mode::Edit = app_state.mode {
                            if let Some(index) = app_state.selected_task {
                                app_state.update_task(app_state.input.clone());
                                app_state.selected_task = Some(index);
                            }
                        } else {
                            app_state.add_task(app_state.input.clone());
                        }
                        app_state.mode = Mode::Normal;
                    }
                    Key::Char(c) => {
                        app_state.input.push(c);
                    }
                    Key::Backspace => {
                        app_state.input.pop();
                    }
                    _ => {}
                },
            }
        }

        terminal.draw(|f| {
            let size = f.size();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(size);

            let (title, input_text) = match app_state.mode {
                Mode::Input => ("Input", format!("Input Mode: {}", app_state.input)),
                Mode::Edit => ("Edit", format!("Editing: {}", app_state.input)),
                Mode::DeleteConfirm => (
                    "Delete",
                    "Press 'd' again to confirm deletion, or any other key to cancel.".to_string(),
                ),
                _ => ("Input", "Press 'n' to add a task".to_string()),
            };

            let input_paragraph = Paragraph::new(input_text)
                .block(Block::default().borders(Borders::ALL).title(title));
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

    app_state.save_tasks("tasks.json");
    terminal.clear()?;
    terminal.set_cursor(0, 0)?;
    terminal.show_cursor()?;
    Ok(())
}
