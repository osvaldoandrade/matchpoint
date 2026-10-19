use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::Model;
use crate::error::Error;

pub(super) struct Command {
    pub name: &'static str,
    pub description: &'static str,
    line: &'static str,
}

impl Command {
    const fn new(name: &'static str, description: &'static str, line: &'static str) -> Self {
        Self {
            name,
            description,
            line,
        }
    }
}

const COMMANDS: &[Command] = &[
    Command::new("/help", "Help and shortcuts", "help"),
    Command::new("/panel", "Switch detail tab", "panel"),
    Command::new("/api", "API settings", "configure"),
    Command::new("/import", "Create a job from CSV", "import-form"),
    Command::new("/threads", "Selected job threads", "threads-form"),
    Command::new(
        "/threads-default",
        "Processing defaults",
        "default-threads-form",
    ),
    Command::new("/checksum", "Check file SHA-256", "job checksum"),
    Command::new("/jobs", "List jobs", "job list"),
    Command::new("/status", "Selected job overview", "job status"),
    Command::new("/run", "Run selected job", "job run"),
    Command::new("/pause", "Pause active job", "job pause"),
    Command::new("/resume", "Resume selected job", "job resume"),
    Command::new("/merge", "Prepare Parquet export", "job merge"),
    Command::new("/merge-partial", "Export uncertain results", "job merge"),
    Command::new("/retry-failed", "Retry final failures", "job retry-failed"),
    Command::new("/events", "Full activity and results", "events-view"),
    Command::new("/motion", "Toggle motion", "motion-toggle"),
    Command::new("/home", "Return home", "home-view"),
    Command::new("/appearance", "Appearance settings", "appearance-form"),
    Command::new("/doctor", "Check data and configuration", "doctor"),
    Command::new("/quit", "Quit safely", "quit"),
];

pub(super) fn suggestions(model: &Model) -> Option<Vec<&'static Command>> {
    let prefix = model.repl.trim_start();
    if model.completion_hidden
        || !prefix.starts_with('/')
        || prefix.chars().any(char::is_whitespace)
        || crate::repl::sensitive(prefix)
    {
        return None;
    }
    Some(
        COMMANDS
            .iter()
            .filter(|cmd| cmd.name.starts_with(prefix))
            .collect(),
    )
}

pub(super) fn shortcut(mut key: KeyEvent) -> KeyEvent {
    if key.modifiers != KeyModifiers::CONTROL {
        return key;
    }
    key.code = match key.code {
        KeyCode::Char('g') => KeyCode::F(1),
        KeyCode::Char('p') => KeyCode::F(2),
        KeyCode::Char('o') => KeyCode::F(3),
        KeyCode::Char('n') => KeyCode::F(4),
        KeyCode::Char('r') => KeyCode::F(5),
        KeyCode::Char('x') => KeyCode::F(6),
        KeyCode::Char('t') => KeyCode::F(7),
        KeyCode::Char('k') => KeyCode::F(8),
        KeyCode::Char('l') => KeyCode::F(9),
        KeyCode::Char('b') => KeyCode::Up,
        KeyCode::Char('f') => KeyCode::Down,
        _ => return key,
    };
    if matches!(key.code, KeyCode::F(_)) {
        key.modifiers = KeyModifiers::NONE;
    }
    key
}

pub(super) fn complete_key(model: &mut Model, key: KeyEvent) -> bool {
    if key.code == KeyCode::Char('u') && key.modifiers == KeyModifiers::CONTROL {
        model.repl.clear();
        model.history_at = None;
        model.completion_hidden = false;
        model.completion_at = 0;
        model.input_cursor = None;
        return true;
    }
    if key.modifiers.is_empty()
        || (key.code == KeyCode::BackTab && key.modifiers == KeyModifiers::SHIFT)
    {
        if let Some(items) = suggestions(model) {
            if menu_key(model, key.code, &items) {
                return true;
            }
        }
    }
    if matches!(key.code, KeyCode::Char(_) | KeyCode::Backspace)
        && !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        model.completion_at = 0;
        model.completion_hidden = false;
    }
    false
}

fn menu_key(model: &mut Model, code: KeyCode, items: &[&Command]) -> bool {
    let index = model.completion_at.min(items.len().saturating_sub(1));
    match code {
        KeyCode::Up | KeyCode::BackTab if !items.is_empty() => {
            model.completion_at = (index + items.len() - 1) % items.len();
            true
        }
        KeyCode::Down if !items.is_empty() => {
            model.completion_at = (index + 1) % items.len();
            true
        }
        KeyCode::Tab | KeyCode::Enter
            if !items.is_empty()
                && (code == KeyCode::Tab || model.repl.trim() != items[index].name) =>
        {
            model.repl = items[index].name.into();
            model.history_at = None;
            model.input_cursor = None;
            model.completion_hidden = true;
            true
        }
        KeyCode::Esc => {
            model.completion_hidden = true;
            true
        }
        _ => matches!(
            code,
            KeyCode::Up | KeyCode::Down | KeyCode::Tab | KeyCode::BackTab
        ),
    }
}

pub(super) fn select_job(model: &mut Model, code: KeyCode) {
    if model.jobs.is_empty() {
        return;
    }
    let index = model
        .jobs
        .iter()
        .position(|job| Some(&job.id) == model.selected_job.as_ref())
        .unwrap_or(0);
    let next = if code == KeyCode::Up {
        index.saturating_sub(1)
    } else {
        (index + 1).min(model.jobs.len() - 1)
    };
    model.selected_job = Some(model.jobs[next].id.clone());
    model.batch_offset = 0;
    model.scroll = 0;
}

pub(super) fn submit_input(model: &mut Model) -> Option<String> {
    let line = model.repl.trim().to_string();
    model.repl.clear();
    model.history_at = None;
    model.completion_hidden = false;
    model.completion_at = 0;
    model.input_cursor = None;
    if line.is_empty() {
        return None;
    }
    if !crate::repl::sensitive(&line) {
        model.history.push(line.clone());
        if model.history.len() > 200 {
            model.history.remove(0);
        }
    } else {
        model.message = "Sensitive command omitted from history".into();
    }
    Some(line)
}

pub(super) fn resolve(model: &mut Model, line: &str) -> Result<Option<String>, Error> {
    let Some(stripped) = line.strip_prefix('/') else {
        return Ok(Some(line.into()));
    };
    let Some(cmd) = COMMANDS.iter().find(|cmd| cmd.name == line) else {
        return Ok(Some(stripped.into()));
    };
    match cmd.name {
        "/panel" => {
            model.panel = (model.panel + 1) % 5;
            model.scroll = 0;
            Ok(None)
        }
        "/status" | "/run" | "/pause" | "/resume" | "/merge" | "/merge-partial" | "/checksum"
        | "/retry-failed" => {
            let target = if cmd.name == "/pause" {
                model.active_job.as_ref().or(model.selected_job.as_ref())
            } else {
                model.selected_job.as_ref()
            };
            let id =
                target.ok_or_else(|| Error::Command("N: create a job; arrows select.".into()))?;
            let line = format!("{} {id}", cmd.line);
            if matches!(cmd.name, "/merge" | "/merge-partial") {
                model.repl = format!(
                    "{line} --format parquet{}",
                    if cmd.name == "/merge-partial" {
                        " --allow-unknown"
                    } else {
                        ""
                    }
                );
                model.input_cursor = None;
                model.message = if cmd.name == "/merge-partial" {
                    "PARTIAL: UNKNOWN is uncertain. Review and press Enter to export."
                } else {
                    "Review the format (parquet or csv); Enter exports."
                }
                .into();
                Ok(None)
            } else {
                Ok(Some(line))
            }
        }
        _ => Ok(Some(cmd.line.into())),
    }
}
