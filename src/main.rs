use chrono::Datelike;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self};
use std::io::{BufReader, Stdout};
use termion::{event::Key, input::TermRead, raw::IntoRawMode, raw::RawTerminal};
use tui::{
    backend::{Backend, TermionBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Text,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};

#[derive(Serialize, Deserialize)]
struct Task {
    description: String,
    completed: bool,

    #[serde(
        serialize_with = "serialize_date",
        deserialize_with = "deserialize_date",
        default
    )]
    deadline: Option<NaiveDateTime>,
}

impl Task {
    fn new(description: String, deadline: Option<NaiveDateTime>) -> Task {
        Task {
            description,
            completed: false,
            deadline,
        }
    }

    fn toggle_completed(&mut self) {
        self.completed = !self.completed;
    }
}

#[derive(PartialEq)]
enum Mode {
    Normal,
    Input,
    Edit,
    DeleteConfirm,
    DeadlineInput,
}

struct AppState {
    tasks: Vec<Task>,
    input: String,
    mode: Mode,
    selected_task: Option<usize>,
    temp_description: String,
    setting_deadline: bool,
}

impl AppState {
    fn new() -> AppState {
        AppState {
            tasks: Vec::new(),
            input: String::new(),
            mode: Mode::Normal,
            selected_task: Some(0),
            temp_description: String::new(),
            setting_deadline: false,
        }
    }

    fn add_task(&mut self, description: String, deadline: Option<NaiveDateTime>) {
        let task = Task::new(description, deadline);
        self.tasks.push(task);
    }

    fn update_task(&mut self, description: String, deadline: Option<NaiveDateTime>) {
        if let Some(index) = self.selected_task {
            if let Some(task) = self.tasks.get_mut(index) {
                task.description = description;
                task.deadline = deadline;
            }
        }
    }

    fn delete_task(&mut self) {
        if let Some(index) = self.selected_task {
            if index < self.tasks.len() {
                self.tasks.remove(index);
            }
        }
    }

    fn load_tasks(&mut self, file_path: &str) -> Result<(), io::Error> {
        let file = match File::open(file_path) {
            Ok(f) => f,
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                let file = File::create(file_path)?;
                serde_json::to_writer(&file, &Vec::<Task>::new())?;
                return Ok(());
            }
            Err(e) => return Err(e),
        };

        let reader = BufReader::new(file);
        match serde_json::from_reader(reader) {
            Ok(tasks) => {
                self.tasks = tasks;
                Ok(())
            }
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }

    fn save_tasks(&self, file_path: &str) -> Result<(), io::Error> {
        let file = File::create(file_path)?;

        let active_tasks: Vec<&Task> = self.tasks.iter().filter(|t| !t.completed).collect();
        serde_json::to_writer(file, &active_tasks)?;

        Ok(())
    }
}

fn calculate_deadline(option: &str) -> Option<NaiveDateTime> {
    let today = chrono::Local::now().date_naive();
    match option {
        "Today" => Some(today.and_hms_opt(0, 0, 0).unwrap()),
        "Tomorrow" => Some(
            (today + chrono::Duration::days(1))
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        ),
        "This Week" => {
            let days_until_end_of_week = 6 - today.weekday().num_days_from_sunday() as i64;
            Some(
                (today + chrono::Duration::days(days_until_end_of_week))
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
            )
        }
        "This Month" => {
            let first_day_next_month = today
                .with_day(0)
                .and_then(|date| date.checked_add_signed(chrono::Duration::days(1)));
            first_day_next_month.map(|date| {
                (date - chrono::Duration::days(1))
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
            })
        }
        _ => None,
    }
}

fn main() -> Result<(), io::Error> {
    let mut terminal = initialize_terminal()?;

    let mut app_state = AppState::new();
    if let Err(e) = app_state.load_tasks("tasks.json") {
        eprintln!("Error loading tasks: {}", e);
    };
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

    app_state.save_tasks("tasks.json")?;
    terminal.clear()?;
    terminal.set_cursor(0, 0)?;
    terminal.show_cursor()?;
    Ok(())
}

fn render_tasks<B: Backend>(f: &mut Frame<B>, app_state: &AppState, chunk: Rect) {
    let today = chrono::Local::now().naive_local();
    let tasks: Vec<ListItem> = app_state
        .tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let is_selected = Some(i) == app_state.selected_task;
            let is_overdue = task
                .deadline
                .map_or(false, |d| d < today.into() && !task.completed);

            let base_style = if is_overdue {
                Style::default().fg(Color::Red)
            } else if task.completed && !is_selected {
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::CROSSED_OUT)
            } else if is_selected {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let content = Text::styled(task.description.as_str(), base_style);
            ListItem::new(content)
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
        Mode::DeadlineInput => {
            let deadline_options = "1: Today, 2: Tomorrow, 3: This Week, 4: This Month";
            ("Select Deadline", deadline_options.to_string())
        }
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
            Key::Char('c') if app_state.selected_task.is_some() => {
                if let Some(index) = app_state.selected_task {
                    if let Some(task) = app_state.tasks.get_mut(index) {
                        task.toggle_completed();
                    }
                }
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
                if !app_state.setting_deadline {
                    app_state.temp_description = app_state.input.clone();
                    app_state.input.clear();
                    app_state.mode = Mode::DeadlineInput;
                }
            }
            Key::Char(c) => {
                app_state.input.push(c);
            }
            Key::Backspace => {
                app_state.input.pop();
            }
            _ => {}
        },
        Mode::DeadlineInput => match key {
            Key::Char('1') => app_state.input = "Today".to_string(),
            Key::Char('2') => app_state.input = "Tomorrow".to_string(),
            Key::Char('3') => app_state.input = "This Week".to_string(),
            Key::Char('4') => app_state.input = "This Month".to_string(),
            Key::Char('q') | Key::Esc => {
                app_state.mode = Mode::Normal;
            }
            Key::Char('\n') => {
                let deadline_option = app_state.input.clone();
                let deadline = calculate_deadline(&deadline_option);

                let description = std::mem::take(&mut app_state.temp_description);
                if let Mode::Edit = app_state.mode {
                    app_state.update_task(description, deadline);
                } else {
                    app_state.add_task(description, deadline);
                }

                app_state.mode = Mode::Normal;
            }
            _ => {}
        },
    }
    true
}

fn serialize_date<S>(date: &Option<NaiveDateTime>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match date {
        Some(d) => serializer.serialize_str(&d.format("%Y-%m-%d %H:%M:%S").to_string()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_date<'de, D>(deserializer: D) -> Result<Option<NaiveDateTime>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(str) => NaiveDateTime::parse_from_str(&str, "%Y-%m-%d %H:%M:%S")
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}
