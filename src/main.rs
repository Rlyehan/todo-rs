use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self};
use std::io::{BufReader, Stdout};
use termion::{event::Key, input::TermRead, raw::IntoRawMode, raw::RawTerminal};
use tui::{
    backend::{Backend, TermionBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
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
    let mut terminal = initialize_terminal()?;

    let mut app_state = AppState::new();
    app_state.load_tasks("tasks.json");
    println!("Loaded {} tasks", app_state.tasks.len());
    let mut keys = io::stdin().keys();

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = create_layout(size);

            render_input_paragraph(f, &app_state, chunks[0]);
            render_tasks(f, &app_state, chunks[1]);
        })?;

        if let Some(Ok(key)) = keys.next() {
            if !process_key_event(key, &mut app_state) {
                break;
            };
        }
    }

    app_state.save_tasks("tasks.json");
    terminal.clear()?;
    terminal.set_cursor(0, 0)?;
    terminal.show_cursor()?;
    Ok(())
}

fn render_tasks<B: Backend>(f: &mut Frame<B>, app_state: &AppState, chunk: Rect) {
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

    let tasks_list = List::new(tasks).block(Block::default().borders(Borders::ALL).title("Tasks"));
    f.render_widget(tasks_list, chunk);
}

fn render_input_paragraph<B: Backend>(f: &mut Frame<B>, app_state: &AppState, chunk: Rect) {
    let (title, input_text) = match app_state.mode {
        Mode::Input => ("Input", format!("Input Mode: {}", app_state.input)),
        Mode::Edit => ("Edit", format!("Editing: {}", app_state.input)),
        Mode::DeleteConfirm => (
            "Delete",
            "Press 'd' again to confirm deletion, or any other key to cancel.".to_string(),
        ),
        _ => ("Input", "Press 'n' to add a task".to_string()),
    };

    let input_paragraph =
        Paragraph::new(input_text).block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(input_paragraph, chunk);
}

fn initialize_terminal() -> Result<Terminal<TermionBackend<RawTerminal<Stdout>>>, io::Error> {
    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    Ok(terminal)
}

fn create_layout(size: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(size)
}

fn process_key_event(key: Key, app_state: &mut AppState) -> bool {
    match app_state.mode {
        Mode::Normal => match key {
            Key::Char('q') => {
                return false;
            }
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
            _ => {
                app_state.mode = Mode::Normal;
            }
        },
        Mode::Input | Mode::Edit => match key {
            Key::Char('\n') => {
                if !app_state.input.is_empty() {
                    if let Mode::Edit = app_state.mode {
                        app_state.update_task(app_state.input.clone());
                    } else {
                        app_state.add_task(app_state.input.clone());
                    }
                }
                app_state.mode = Mode::Normal;
                app_state.input.clear();
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
    true
}
