use clap::Parser;
use color_eyre::Result;
use crossterm::ExecutableCommand;
use crossterm::cursor::Show;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};
use std::env;
use std::fs;
use std::io::stdout;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Command to un-forget
    args: Vec<String>,
}

static PATHS: &[&str] = &[".bash_history", ".zsh_history"];

struct App {
    should_quit: bool,
    command_history: Vec<String>,
    list_state: ListState,
    search_input: String,
    filtered_commands: Vec<usize>,
    selected_command: Option<String>,
}

impl App {
    fn new(initial_args: Vec<String>) -> Result<Self> {
        let command_history = Self::load_history()?;
        let search_input = initial_args.join(" ");
        let filtered_commands = Self::filter_commands(&command_history, &search_input);

        let mut list_state = ListState::default();
        if !filtered_commands.is_empty() {
            list_state.select(Some(0));
        }

        Ok(Self {
            should_quit: false,
            command_history,
            list_state,
            search_input,
            filtered_commands,
            selected_command: None,
        })
    }

    fn load_history() -> Result<Vec<String>> {
        let data_home = env::var_os("HOME").expect("HOME isn't set");
        let base_path = PathBuf::from(data_home);

        for path in PATHS {
            let full_path = base_path.join(path);
            if full_path.exists() {
                let content = fs::read_to_string(full_path)?;
                let mut commands: Vec<String> = content
                    .lines()
                    .map(|line| {
                        // zsh starts with :
                        if line.starts_with(":") {
                            if let Some(pos) = line.find(';') {
                                return line[pos + 1..].to_string();
                            }
                        }
                        line.to_string()
                    })
                    .filter(|cmd| !cmd.trim().is_empty())
                    .collect();

                commands.reverse();
                let mut seen = std::collections::HashSet::new();
                commands.retain(|cmd| seen.insert(cmd.clone()));

                return Ok(commands);
            }
        }

        Ok(Vec::new())
    }
    fn filter_commands(commands: &[String], query: &str) -> Vec<usize> {
        if query.is_empty() {
            return (0..commands.len()).collect();
        }

        commands
            .iter()
            .enumerate()
            .filter(|(_, cmd)| cmd.to_lowercase().contains(&query.to_lowercase()))
            .map(|(i, _)| i)
            .collect()
    }

    fn update_filter(&mut self) {
        self.filtered_commands = Self::filter_commands(&self.command_history, &self.search_input);

        // Reset selection to first item
        if !self.filtered_commands.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous();
            }
            KeyCode::Char(c) => {
                self.search_input.push(c);
                self.update_filter();
            }
            KeyCode::Backspace => {
                self.search_input.pop();
                self.update_filter();
            }
            KeyCode::Enter => {
                if let Some(selected) = self.list_state.selected() {
                    if let Some(&cmd_idx) = self.filtered_commands.get(selected) {
                        self.selected_command = Some(self.command_history[cmd_idx].clone());
                    }
                }
                self.should_quit = true;
            }
            _ => {}
        }
    }

    fn select_next(&mut self) {
        if self.filtered_commands.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.filtered_commands.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn select_previous(&mut self) {
        if self.filtered_commands.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.filtered_commands.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn render(&mut self, frame: &mut Frame) {
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .spacing(1);
        let [top, search, main, bottom] = vertical.areas(frame.area());

        let title = Line::from_iter([
            Span::from("I f****** forgot").bold(),
            Span::from(" (Press 'q' to quit, ↑↓ to navigate, Enter to select)"),
        ]);
        frame.render_widget(title.centered(), top);

        let search_block = Block::bordered().title("Search").style(Style::new().cyan());
        let search_text = Paragraph::new(self.search_input.as_str()).block(search_block);
        frame.render_widget(search_text, search);

        self.render_command_list(frame, main);

        // Status bar
        let status = format!(
            "{} / {} commands",
            self.filtered_commands.len(),
            self.command_history.len()
        );
        frame.render_widget(Line::from(status).centered().dim(), bottom);
    }

    fn render_command_list(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .filtered_commands
            .iter()
            .map(|&idx| {
                let cmd = &self.command_history[idx];
                ListItem::new(cmd.as_str())
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::bordered()
                    .title("Command History")
                    .style(Style::new().blue()),
            )
            .highlight_style(
                Style::new()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }
}

fn parse_command_string(input: &str) -> (String, Vec<String>) {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for c in input.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' if !in_quotes => {
                if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    let program = parts.first().unwrap_or(&String::new()).clone();
    let args = parts.into_iter().skip(1).collect();

    (program, args)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    color_eyre::install()?;

    let mut app = App::new(cli.args)?;
    let mut terminal = ratatui::init();

    loop {
        terminal.draw(|frame| app.render(frame))?;

        if let Event::Key(key) = event::read()? {
            app.handle_key(key);
        }

        if app.should_quit {
            break;
        }
    }

    ratatui::restore();
    let _ = stdout().execute(Show);

    if let Some(command) = app.selected_command {
        let (com, args) = parse_command_string(&command);
        let err = Command::new(com).args(args).exec();
        eprintln!("Failed to exec: {}", err);
        std::process::exit(1);
    }

    Ok(())
}
