use std::collections::VecDeque;
use std::io::{self, IsTerminal};

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures_util::StreamExt;

use crate::cli::{execute_collect, parse_line, AuthCmd, Cmd, JobCmd};
use crate::engine::{App, JobStatus, RunOptions, RunReport};
use crate::error::Error;
use crate::repl::complete;

pub(crate) mod appearance;
mod art;
mod brand;
mod commands;
mod configure;
mod copy;
mod feedback;
pub(crate) mod forms;
pub(crate) mod palette;
pub(crate) mod session;
mod settings;
pub(crate) mod terminal;
pub(crate) mod view;
mod work;
pub use art::SceneKind;
use configure::ConfigForm;
use session::{load_session, save_session};
use terminal::CrosstermTerm;
pub use terminal::{Guard, TermMode};
pub use view::render;
pub use work::perform_pending_work;

pub struct Model {
    /// Interactive launch shows this once; programmatic models start at Home.
    pub splash: bool,
    pub home: bool,
    pub matching_mode: bool,
    pub home_selection: usize,
    pub command_mode: bool,
    pub jobs_line: String,
    pub dashboard: String,
    pub batches: String,
    pub events: VecDeque<String>,
    pub repl: String,
    pub message: String,
    pub help: bool,
    pub panel: usize,
    pub filter: String,
    pub history: Vec<String>,
    pub history_at: Option<usize>,
    pub auth_line: String,
    pub jobs: Vec<JobStatus>,
    pub selected_job: Option<String>,
    pub progress: f64,
    pub api_ready: bool,
    pub busy: bool,
    pub active_job: Option<String>,
    pub selected_config: Option<crate::jobcfg::JobConfig>,
    pub job_configs: Vec<crate::jobcfg::JobConfig>,
    pub batch_items: Vec<crate::store::BatchInfo>,
    pub batch_offset: u32,
    pub job_events: Vec<String>,
    pub checksum: Option<crate::engine::ChecksumReport>,
    pub error: Option<feedback::Feedback>,
    pub scroll: u16,
    pub animation: u64,
    pub motion: bool,
    pub working: bool,
    pub work_title: String,
    pub input_cursor: Option<usize>,
    pub message_job: Option<String>,
    pub viewport: (u16, u16),
    pub help_page: usize,
    pub animation_speed: u8,
    pub font_size: u16,
    pub font_vga: bool,
    pub scene: SceneKind,
    interaction_at: Option<u64>,
    settings: Option<settings::Settings>,
    form: Option<ConfigForm>,
    thread_form: Option<forms::ThreadForm>,
    import_form: Option<forms::ImportForm>,
    import_draft: Option<forms::ImportForm>,
    export_form: Option<forms::ExportForm>,
    appearance_form: Option<appearance::AppearanceForm>,
    pending_work: Option<work::Work>,
    completion_at: usize,
    completion_hidden: bool,
}

impl Model {
    pub fn new() -> Self {
        let mut events = VecDeque::new();
        events.push_back("Session ready".into());
        Self {
            splash: false,
            home: true,
            matching_mode: false,
            home_selection: 0,
            command_mode: false,
            jobs_line: "No jobs".into(),
            dashboard: "Select a job".into(),
            batches: "Batches".into(),
            events,
            repl: String::new(),
            message: String::new(),
            help: false,
            panel: 0,
            filter: "all".into(),
            history: Vec::new(),
            history_at: None,
            auth_line: "API: S opens Settings".into(),
            jobs: Vec::new(),
            selected_job: None,
            progress: 0.0,
            api_ready: false,
            busy: false,
            active_job: None,
            selected_config: None,
            job_configs: Vec::new(),
            batch_items: Vec::new(),
            batch_offset: 0,
            job_events: Vec::new(),
            checksum: None,
            error: None,
            scroll: 0,
            animation: 0,
            motion: true,
            working: false,
            work_title: String::new(),
            input_cursor: None,
            message_job: None,
            viewport: (80, 24),
            help_page: 0,
            animation_speed: 2,
            font_size: 16,
            font_vga: true,
            scene: SceneKind::Tesseract,
            interaction_at: None,
            settings: None,
            form: None,
            thread_form: None,
            import_form: None,
            import_draft: None,
            export_form: None,
            appearance_form: None,
            pending_work: None,
            completion_at: 0,
            completion_hidden: false,
        }
    }

    pub(crate) fn touch(&mut self) {
        self.interaction_at = Some(self.animation);
    }

    pub fn push_event(&mut self, line: String) {
        self.events.push_back(feedback::safe(&line));
        while self.events.len() > 200 {
            self.events.pop_front();
        }
    }

    pub fn show_error(&mut self, error: &Error) {
        let mut report = feedback::Feedback::from_error(error);
        if let Some(settings) = &mut self.settings {
            if let Error::Msg(text) = error {
                if text.starts_with("Enter a client secret") {
                    settings.tab = 1;
                    settings.focus = Some(0);
                    settings.api.focus = 3;
                } else if settings.tab == 2 {
                    let field = if text.starts_with("Threads must") {
                        Some(0)
                    } else if text.starts_with("RPS must") {
                        Some(1)
                    } else if text.starts_with("Batch size must") {
                        Some(2)
                    } else {
                        None
                    };
                    if field.is_some() {
                        settings.focus = field;
                    }
                }
            }
        }
        if let Some(settings) = &mut self.settings {
            settings.error_group = Some(if settings.tab < 2 { 0 } else { settings.tab });
        }
        if self.form.is_some()
            && matches!(
                error,
                Error::AuthMissing | Error::TokenInvalid(_) | Error::InsecureUrl | Error::UrlSecret
            )
        {
            report.recovery = "Fix the field; Ctrl+S saves.".into();
        } else if self.import_form.is_some() && matches!(error, Error::Io(_)) {
            report.recovery = "Fix the path; Enter creates.".into();
        } else if self.export_form.is_some() && matches!(error, Error::UnknownBlocks { .. }) {
            report.recovery = "Tab: partial · Space: toggle.".into();
        }
        self.message = format!("ERROR: {}", report.title);
        self.push_event(format!(
            "ERROR: {}\n{}\nAction: {}\nDiagnostic: {}",
            report.title,
            report.detail,
            report.recovery,
            feedback::safe(&error.to_string())
        ));
        self.error = Some(report);
        self.message_job = self.selected_job.clone();
    }
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

fn command_error(model: &mut Model, error: &Error, line: &str, submitted: bool) {
    model.show_error(error);
    if submitted && !crate::repl::sensitive(line) {
        model.repl = line.into();
        model.input_cursor = None;
        model.completion_hidden = true;
    }
}

/// Applies a runner result while retaining its origin if selection changed.
pub fn finish_run(model: &mut Model, outcome: Result<RunReport, Error>) {
    let origin = model.active_job.take();
    model.busy = false;
    let id = origin
        .as_deref()
        .map(|id| id.get(..8).unwrap_or(id))
        .unwrap_or("?");
    match outcome {
        Ok(report) => {
            model.error = None;
            model.message = format!("Job {id}: {}.", view::state_label(&report.state));
            model.push_event(model.message.clone());
        }
        Err(err) => {
            model.show_error(&err);
            if let Some(settings) = &mut model.settings {
                settings.error_group = None;
            }
            if let Some(feedback) = &mut model.error {
                feedback.title = format!("{id}: {}", feedback.title);
            }
            model.push_event(format!("Job {id}: {}", model.message));
        }
    }
    // A final result can refer to an active job that is no longer selected.
    model.message_job = None;
}

pub fn apply_paste(model: &mut Model, value: &str) {
    if model.splash {
        return;
    }
    if let Some(settings) = &mut model.settings {
        settings.paste(value);
    } else if let Some(form) = &mut model.form {
        form.paste(value);
    } else if let Some(form) = &mut model.import_form {
        form.paste(value);
    } else if let Some(form) = &mut model.thread_form {
        forms::paste(&mut form.value, &mut form.cursor, value, 8);
    } else if model.command_mode
        && model.export_form.is_none()
        && model.appearance_form.is_none()
        && !model.help
    {
        forms::paste(&mut model.repl, &mut model.input_cursor, value, 8192);
    } else {
        return;
    }
    model.error = None;
}

pub fn apply_key(model: &mut Model, key: KeyEvent) -> Option<String> {
    if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
        return None;
    }
    model.interaction_at = Some(model.animation);
    if model.splash {
        return view::splash::key(model, key);
    }
    if let Some(settings) = &mut model.settings {
        let before = (settings.tab, settings.fields());
        let action = settings.key(key).map(str::to_string);
        if settings.tab == before.0 && settings.fields() != before.1 {
            model.error = None;
            settings.error_group = None;
        }
        if action.as_deref() == Some("close-settings") {
            let dirty = model
                .settings
                .take()
                .is_some_and(|s| s.dirty.iter().any(|v| *v));
            model.error = None;
            model.message_job = None;
            model.message = if dirty {
                "Settings closed; unsaved edits discarded."
            } else {
                "Settings closed."
            }
            .into();
            return None;
        }
        return action;
    }
    if model.thread_form.is_some()
        || model.import_form.is_some()
        || model.export_form.is_some()
        || model.appearance_form.is_some()
    {
        match key.code {
            KeyCode::Esc => {
                model.thread_form = None;
                model.import_form = None;
                model.export_form = None;
                model.appearance_form = None;
                model.error = None;
                model.message = "Canceled; no changes saved.".into();
            }
            KeyCode::Enter => {
                return Some(
                    if let Some(form) = &model.appearance_form {
                        if form.focus == 3 {
                            "apply-ghostty"
                        } else {
                            "save-appearance"
                        }
                    } else if model.thread_form.is_some() {
                        "save-threads"
                    } else if model.export_form.is_some() {
                        "save-export"
                    } else {
                        "save-import"
                    }
                    .into(),
                )
            }
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                return Some("quit".into())
            }
            _ => {
                if let Some(form) = &mut model.thread_form {
                    let previous = form.value.clone();
                    forms::edit(&mut form.value, &mut form.cursor, key, 8);
                    if form.value != previous {
                        model.error = None;
                    }
                }
                if let Some(form) = &mut model.import_form {
                    let previous = (form.path.clone(), form.threads.clone(), form.validation);
                    form.key(key);
                    if (form.path.clone(), form.threads.clone(), form.validation) != previous {
                        model.error = None;
                    }
                }
                if let Some(form) = &mut model.export_form {
                    let previous = (form.format_csv, form.partial);
                    form.key(key);
                    if (form.format_csv, form.partial) != previous {
                        model.error = None;
                    }
                }
                if let Some(form) = &mut model.appearance_form {
                    let previous = (form.font_vga, form.font_size, form.speed);
                    form.key(key);
                    if (form.font_vga, form.font_size, form.speed) != previous {
                        model.error = None;
                    }
                }
            }
        }
        return None;
    }
    if model.form.is_some() {
        match key.code {
            KeyCode::Esc => {
                model.form = None;
                model.error = None;
                model.message = "Settings canceled".into();
            }
            KeyCode::Enter => return Some("save-api".into()),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                model.form = None;
                return Some("quit".into());
            }
            _ => {
                if let Some(form) = model.form.as_mut() {
                    let previous = form.fields();
                    form.key(key);
                    if form.fields() != previous {
                        model.error = None;
                    }
                }
            }
        }
        return None;
    }
    if model.help
        && !model.command_mode
        && model.repl.is_empty()
        && !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        if let KeyCode::Char(ch) = key.code {
            let action = match ch.to_ascii_lowercase() {
                'q' => Some("quit"),
                'h' => Some("home-view"),
                'j' => Some("jobs-view"),
                'n' => Some("import-form"),
                's' => Some("settings-view"),
                'r' if !model.home => Some("/run"),
                'p' if !model.home => Some("/pause"),
                't' if !model.home => Some("threads-form"),
                'e' if !model.home => Some("export-form"),
                'k' if !model.home => Some("/checksum"),
                _ => None,
            };
            if let Some(action) = action {
                model.help = false;
                model.scroll = 0;
                return Some(action.into());
            }
        }
    }
    if model.help {
        match key.code {
            KeyCode::Esc | KeyCode::F(1) => {
                model.help = false;
                model.scroll = 0;
            }
            KeyCode::Char('g') if key.modifiers == KeyModifiers::CONTROL => {
                model.help = false;
                model.scroll = 0;
            }
            KeyCode::Down | KeyCode::PageDown => {
                model.scroll = model
                    .scroll
                    .saturating_add(if key.code == KeyCode::Down { 1 } else { 5 })
                    .min(500)
            }
            KeyCode::Up | KeyCode::PageUp => {
                model.scroll =
                    model
                        .scroll
                        .saturating_sub(if key.code == KeyCode::Up { 1 } else { 5 })
            }
            KeyCode::Home => model.scroll = 0,
            KeyCode::Tab | KeyCode::Right => {
                model.help_page = (model.help_page + 1) % 4;
                model.scroll = 0;
            }
            KeyCode::BackTab | KeyCode::Left => {
                model.help_page = (model.help_page + 3) % 4;
                model.scroll = 0;
            }
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                return Some("quit".into())
            }
            _ => {}
        }
        return None;
    }
    let key = if !model.command_mode
        && model.repl.is_empty()
        && !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        if let KeyCode::Char(ch) = key.code {
            KeyEvent::new(KeyCode::Char(ch.to_ascii_lowercase()), KeyModifiers::NONE)
        } else if key.code == KeyCode::BackTab {
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE)
        } else {
            key
        }
    } else {
        key
    };
    // Menus are the primary interaction. Slash opens the optional expert prompt.
    if !model.command_mode && model.repl.is_empty() && key.modifiers.is_empty() {
        if key.code == KeyCode::Char('/') {
            model.command_mode = true;
            model.repl = "/".into();
            return None;
        }
        if key.code == KeyCode::Char('?') {
            model.help = true;
            model.scroll = 0;
            return None;
        }
        if model.home {
            match key.code {
                KeyCode::Left | KeyCode::Up | KeyCode::BackTab => {
                    model.home_selection = (model.home_selection + 2) % 3;
                    return None;
                }
                KeyCode::Right | KeyCode::Down | KeyCode::Tab => {
                    model.home_selection = (model.home_selection + 1) % 3;
                    return None;
                }
                KeyCode::Enter => return Some(copy::HOME_COMMANDS[model.home_selection].into()),
                KeyCode::Char(ch @ '1'..='3') => {
                    return Some(copy::HOME_COMMANDS[(ch as u8 - b'1') as usize].into())
                }
                KeyCode::Char('q') => return Some("quit".into()),
                KeyCode::Char('j') => return Some("jobs-view".into()),
                KeyCode::Char('n') => return Some("import-form".into()),
                KeyCode::Char('s') => return Some("settings-view".into()),
                KeyCode::Char('d') => return Some("doctor".into()),
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Up | KeyCode::Down => {
                    commands::select_job(model, key.code);
                    return None;
                }
                KeyCode::Tab | KeyCode::Right => {
                    model.panel = if model.panel == 0 {
                        1
                    } else {
                        model.panel % 4 + 1
                    };
                    model.scroll = 0;
                    return None;
                }
                KeyCode::BackTab | KeyCode::Left => {
                    model.panel = if model.panel <= 1 { 4 } else { model.panel - 1 };
                    model.scroll = 0;
                    return None;
                }
                KeyCode::Char('h') => return Some("home-view".into()),
                KeyCode::Char('i' | 'n') => return Some("import-form".into()),
                KeyCode::Enter => {
                    model.panel = 1;
                    return None;
                }
                KeyCode::Char('a') => return Some("configure".into()),
                KeyCode::Char('t') => return Some("threads-form".into()),
                KeyCode::Char('k') => return Some("/checksum".into()),
                KeyCode::Char('e') => return Some("export-form".into()),
                KeyCode::Char('r') => return Some("/run".into()),
                KeyCode::Char('p') => return Some("/pause".into()),
                KeyCode::Char('s') => return Some("settings-view".into()),
                KeyCode::Char('q') => return Some("quit".into()),
                KeyCode::Esc => {
                    if model.viewport.0 < 80 && model.panel > 0 {
                        model.panel = 0;
                    } else {
                        model.home = true;
                    }
                    model.error = None;
                    return None;
                }
                _ => {}
            }
        }
    }
    let key = commands::shortcut(key);
    if commands::complete_key(model, key) {
        return None;
    }
    match key.code {
        KeyCode::F(1) => {
            model.help = true;
            model.scroll = 0;
            model.help_page = 0;
        }
        KeyCode::F(2) => {
            model.panel = if model.panel == 0 {
                1
            } else {
                model.panel % 4 + 1
            };
            model.scroll = 0;
        }
        KeyCode::F(3) => return Some("configure".into()),
        KeyCode::F(4) => return Some("import-form".into()),
        KeyCode::F(7) => return Some("threads-form".into()),
        KeyCode::F(8) => return Some("/checksum".into()),
        KeyCode::F(9) => return Some("events-view".into()),
        KeyCode::F(5) | KeyCode::F(6) => {
            let target = if key.code == KeyCode::F(6) {
                model.active_job.as_ref().or(model.selected_job.as_ref())
            } else {
                model.selected_job.as_ref()
            };
            if let Some(id) = target {
                return Some(format!(
                    "job {} {id}",
                    if key.code == KeyCode::F(5) {
                        "run"
                    } else {
                        "pause"
                    }
                ));
            }
            model.message = "N: create a job; arrows select.".into();
        }
        KeyCode::Up | KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
            commands::select_job(model, key.code);
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Some("quit".into())
        }
        KeyCode::Tab => {
            if let Some(next) = complete(&model.repl) {
                model.repl = next;
            }
        }
        KeyCode::Enter => return commands::submit_input(model),
        KeyCode::PageDown => {
            if model.panel == 2 {
                model.batch_offset = model
                    .batch_offset
                    .saturating_add(view::batch_page_size(model));
            } else {
                model.scroll = model.scroll.saturating_add(view::detail_scroll_step(model));
            }
        }
        KeyCode::PageUp => {
            if model.panel == 2 {
                model.batch_offset = model
                    .batch_offset
                    .saturating_sub(view::batch_page_size(model));
            } else {
                model.scroll = model.scroll.saturating_sub(view::detail_scroll_step(model));
            }
        }
        KeyCode::Up => {
            if model.history.is_empty() {
                return None;
            }
            let next = match model.history_at {
                None => model.history.len() - 1,
                Some(0) => 0,
                Some(i) => i - 1,
            };
            model.history_at = Some(next);
            model.repl = model.history[next].clone();
            model.input_cursor = None;
        }
        KeyCode::Down => {
            if let Some(index) = model.history_at {
                let next = index + 1;
                model.history_at = (next < model.history.len()).then_some(next);
                model.repl = model.history.get(next).cloned().unwrap_or_default();
                model.input_cursor = None;
            }
        }
        KeyCode::Esc => {
            model.repl.clear();
            model.help = false;
            model.error = None;
            model.input_cursor = None;
            model.command_mode = false;
        }
        _ if model.command_mode || !model.repl.is_empty() => {
            forms::edit(&mut model.repl, &mut model.input_cursor, key, 8192)
        }
        _ => {}
    }
    None
}

pub async fn launch(app: &App) -> Result<(), Error> {
    crate::matching::ui::launch(app).await
}

/// Historical library runtime, retained for recovery tools; executable entrypoints use matching.
pub async fn launch_legacy(app: &App) -> Result<(), Error> {
    if !std::io::stdout().is_terminal() {
        return Err(Error::NotATty);
    }
    let mut guard = CrosstermTerm::enter()?;
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = CrosstermTerm::restore();
        previous(info);
    }));
    let mut model = Model::new();
    load_session(app, &mut model)?;
    model.splash = true;
    if let Err(err) = refresh(app, &mut model).await {
        model.show_error(&err);
    }
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(io::stdout()))
            .map_err(|e| Error::Msg(e.to_string()))?;
    let mut events = EventStream::new();
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(33));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut running: Option<tokio::task::JoinHandle<Result<RunReport, Error>>> = None;
    let mut local: Option<tokio::task::JoinHandle<Result<work::WorkResult, Error>>> = None;
    let mut run_stop: Option<tokio::sync::watch::Sender<bool>> = None;
    let mut stop = false;
    while !stop || running.is_some() || local.is_some() {
        if let Err(err) = terminal.draw(|frame| {
            model.viewport = (frame.area().width, frame.area().height);
            render(frame, &model)
        }) {
            if let Some(tx) = &run_stop {
                let _ = tx.send(true);
            }
            if let Some(handle) = running.take() {
                let _ = handle.await;
            }
            if let Some(handle) = local.take() {
                let _ = handle.await;
            }
            return Err(Error::Msg(err.to_string()));
        }
        tokio::select! {
            event = events.next(), if !stop => {
                match event {
                    Some(Ok(Event::Key(key))) => {
                        let before = (model.selected_job.clone(),model.batch_offset,model.settings.is_some());
                        let mut action_refresh=false;
                        let submitted = model.command_mode && key.code == KeyCode::Enter && model.settings.is_none() && model.form.is_none() && model.thread_form.is_none() && model.import_form.is_none() && model.export_form.is_none() && model.appearance_form.is_none();
                        if let Some(line) = apply_key(&mut model, key) {
                            action_refresh=true;
                            if matches!(line.as_str(), "quit" | "/quit") {
                                stop = true;
                                model.form = None; model.thread_form = None; model.import_form = None;
                                model.message = "Closing: waiting for requests and local writes…".into();
                                if let Some(tx) = &run_stop { let _ = tx.send(true); }
                            } else {
                                match run_repl_line(app, &mut model, &line).await {
                                    Ok(Some(id)) => {
                                        let (tx, rx) = tokio::sync::watch::channel(false);
                                        run_stop = Some(tx);
                                        let owned = app.clone();
                                        model.active_job = Some(app.status(&id)?.id);
                                        running = Some(tokio::spawn(async move { owned.run_job(&id, RunOptions::with_stop(rx)).await }));
                                        model.busy = true;
                                        model.message = "RUNNING".into();
                                    }
                                    Ok(None) => {}
                                    Err(err) => {
                                        command_error(&mut model,&err,&line,submitted);
                                    }
                                }
                                if local.is_none() {
                                    if let Some(work) = model.pending_work.take() {
                                        model.working = true;
                                        model.work_title = work.title().into();
                                        model.message = format!("{}; Ctrl+C waits for local writes.", model.work_title);
                                        let owned = app.clone();
                                        local = Some(tokio::task::spawn_blocking(move || work.execute(&owned)));
                                    }
                                }
                            }
                        }
                        if action_refresh || before!=(model.selected_job.clone(),model.batch_offset,model.settings.is_some()) {
                            if let Err(err) = refresh(app, &mut model).await { model.show_error(&err); }
                        }
                    }
                    Some(Ok(Event::Paste(value))) => apply_paste(&mut model, &value),
                    Some(Ok(Event::Resize(w, h))) => {
                        model.viewport=(w,h);
                        if let Err(err)=refresh(app,&mut model).await {model.show_error(&err);}
                    }
                    Some(Ok(_)) => {}
                    Some(Err(err)) => model.show_error(&Error::Io(err)),
                    None => { stop = true; if let Some(tx) = &run_stop { let _ = tx.send(true); } }
                }
            }
            _ = tokio::signal::ctrl_c(), if !stop => {
                stop = true; model.form = None;
                model.message = "Closing: waiting for requests and local writes…".into();
                if let Some(tx) = &run_stop { let _ = tx.send(true); }
            }
            result = async { running.as_mut().unwrap().await }, if running.is_some() => {
                running = None; run_stop = None;
                let outcome = result.unwrap_or_else(|_| Err(Error::Msg("Run interrupted; /doctor checks local data.".into())));
                finish_run(&mut model, outcome);
                if let Err(err) = refresh(app, &mut model).await { model.show_error(&err); }
            }
            result = async { local.as_mut().unwrap().await }, if local.is_some() => {
                local = None; model.working = false;
                match result {
                    Ok(Ok(result)) => work::accept(&mut model, result),
                    Ok(Err(err)) => model.show_error(&err),
                    Err(_) => model.show_error(&Error::Msg("Local operation interrupted; check Events.".into())),
                }
                if let Err(err) = refresh(app, &mut model).await { model.show_error(&err); }
            }
            _ = tick.tick() => {
                model.animation = model.animation.wrapping_add(1);
                let polling=if model.busy || model.working || model.jobs.iter().any(|job|job.state=="RUNNING") {20}else{60};
                if model.animation.is_multiple_of(polling) && (model.settings.is_none() || model.busy || model.working) {
                    if let Err(err) = refresh(app, &mut model).await { model.show_error(&err); }
                }
            }
        }
    }
    save_session(app, &model)?;
    guard.leave()?;
    Ok(())
}

pub async fn refresh(app: &App, model: &mut Model) -> Result<(), Error> {
    if model
        .message_job
        .as_ref()
        .is_some_and(|id| Some(id) != model.selected_job.as_ref())
    {
        model.message_job = None;
        model.error = None;
        model.message = String::new();
    }
    let jobs = app.list_jobs()?;
    model.job_configs = jobs
        .iter()
        .map(|job| {
            model
                .job_configs
                .iter()
                .find(|cfg| cfg.job_id == job.id)
                .cloned()
                .map(Ok)
                .unwrap_or_else(|| app.job_config(&job.id))
        })
        .collect::<Result<_, _>>()?;
    if !jobs
        .iter()
        .any(|job| Some(&job.id) == model.selected_job.as_ref())
    {
        model.selected_job = jobs.first().map(|job| job.id.clone());
    }
    if model
        .checksum
        .as_ref()
        .is_some_and(|report| Some(&report.job_id) != model.selected_job.as_ref())
    {
        model.checksum = None;
    }
    model.selected_config = model
        .selected_job
        .as_ref()
        .map(|id| app.job_config(id))
        .transpose()?;
    model.jobs_line = if jobs.is_empty() {
        concat!(
            "No jobs yet\n\nN: create a job from CSV\nS: configure Settings\n\n",
            "Rows are grouped into batches."
        )
        .into()
    } else {
        jobs.iter()
            .map(|job| {
                format!(
                    "{} {} {}",
                    if Some(&job.id) == model.selected_job.as_ref() {
                        ">"
                    } else {
                        " "
                    },
                    &job.id[..8],
                    view::state_label(&job.state)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    if let Some(job) = jobs
        .iter()
        .find(|job| Some(&job.id) == model.selected_job.as_ref())
    {
        let counts = &job.counts;
        let total = counts.total();
        let done = counts.succeeded + counts.failed_final + counts.unknown;
        model.progress = if total > 0 {
            done as f64 / total as f64
        } else {
            0.0
        };
        model.dashboard = format!(
            "{} · {}\nProcessed {done}/{total} ({:.0}%)\nPending {} · reserved {}\nSucceeded {} · failed {}\nUncertain UNKNOWN {}\nRetry {} · batches {}",
            &job.id[..8], view::state_label(&job.state), model.progress * 100.0,
            counts.pending,
            counts.in_flight,
            counts.succeeded,
            counts.failed_final,
            counts.unknown,
            counts.retry_wait, job.batches
        );
        model.batch_offset = model.batch_offset.min(job.batches.saturating_sub(1) as u32);
        let batches = app.list_batches(
            &job.id,
            &model.filter,
            view::batch_page_size(model),
            model.batch_offset,
        )?;
        model.batches = if batches.is_empty() {
            "sem Batches".into()
        } else {
            batches
                .iter()
                .map(|b| {
                    format!(
                        "#{} {}\n{}–{} · succeeded {} · failed {}\npending {} · uncertain {}",
                        b.batch_id,
                        b.state,
                        b.row_from,
                        b.row_to,
                        b.succeeded,
                        b.failed,
                        b.pending + b.in_flight + b.retry_wait,
                        b.unknown
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        model.batch_items = batches;
        model.job_events = app
            .events(&job.id)?
            .into_iter()
            .map(|(at, kind, detail)| feedback::safe(&format!("{at} {kind}: {detail}")))
            .collect();
    } else {
        model.dashboard = "Select a job to see progress".into();
        model.batches = "Batches appear after importing CSV".into();
        model.progress = 0.0;
        model.batch_items.clear();
        model.job_events.clear();
    }
    model.jobs = jobs;
    let config = app.api_config()?;
    model.api_ready = !config.client_id.is_empty() && config.remote_enabled;
    model.auth_line = format!(
        "Client ID: {} · remote: {} · secret: hidden\n{}",
        if config.client_id.is_empty() {
            "not configured"
        } else {
            &config.client_id
        },
        if config.remote_enabled {
            "enabled"
        } else {
            "disabled"
        },
        config.process_url
    );
    Ok(())
}

pub async fn run_repl_line(
    app: &App,
    model: &mut Model,
    line: &str,
) -> Result<Option<String>, Error> {
    let Some(line) = commands::resolve(model, line)? else {
        return Ok(None);
    };
    let line = line.as_str();
    let line = if model.settings.is_some() {
        match line {
            "save-api" | "save-appearance" | "save-threads" => "save-settings",
            "apply-ghostty" => "settings-ghostty",
            _ => line,
        }
    } else {
        line
    };
    if matches!(
        line,
        "settings-view" | "configure" | "appearance-form" | "default-threads-form"
    ) {
        let tab = match line {
            "appearance-form" => 3,
            "default-threads-form" => 2,
            _ => 0,
        };
        if let Some(settings) = &mut model.settings {
            settings.tab = tab;
            settings.focus = None;
        } else {
            model.settings = Some(settings::Settings::new(app, model, tab)?);
        }
        model.error = None;
        model.message.clear();
        model.message_job = None;
        model.form = None;
        model.thread_form = None;
        model.import_form = None;
        model.export_form = None;
        model.appearance_form = None;
        model.help = false;
        model.command_mode = false;
        model.repl.clear();
        return Ok(None);
    }
    if line == "close-settings" {
        let dirty = model
            .settings
            .take()
            .is_some_and(|form| form.dirty.iter().any(|dirty| *dirty));
        model.error = None;
        model.message = if dirty {
            "Settings closed; unsaved edits discarded."
        } else {
            "Settings closed."
        }
        .into();
        model.message_job = None;
        return Ok(None);
    }
    if matches!(line, "save-settings" | "settings-ghostty") {
        if let Some(settings) = &model.settings {
            let tab = settings.tab;
            match tab {
                0 | 1 => {
                    if model.busy {
                        return Err(Error::Msg(
                            "Pause the active job and wait before changing credentials.".into(),
                        ));
                    }
                    settings.api.save(app)?;
                }
                2 => app.set_processing_defaults(settings.processing()?)?,
                _ => {
                    model.font_vga = settings.appearance.font_vga;
                    model.font_size = settings.appearance.font_size;
                    model.motion = settings.appearance.speed > 0;
                    model.animation_speed = settings.appearance.speed.max(1);
                    model.scene = settings.appearance.scene;
                    save_session(app, model)?;
                    if line == "settings-ghostty" {
                        appearance::open_ghostty(app, model)?;
                    }
                }
            }
            if let Some(settings) = &mut model.settings {
                settings.error_group = None;
                settings.load_warnings[if tab < 2 { 0 } else { tab }] = None;
                if tab < 2 {
                    settings.dirty[0] = false;
                    settings.dirty[1] = false;
                    settings.api = ConfigForm::new(app.api_config()?);
                } else {
                    settings.dirty[tab] = false;
                }
            }
            model.error = None;
            model.message_job = None;
            model.message = match tab {
                0 | 1 => "API profile saved. Credentials protected in Keychain.",
                2 => "Processing defaults saved for new jobs.",
                _ if line == "settings-ghostty" => "Ghostty opened. Q closes this window.",
                _ => "Appearance saved. Font changes apply to a new Ghostty window.",
            }
            .into();
            if let Some(settings) = &mut model.settings {
                settings.notice = Some((if tab < 2 { 0 } else { tab }, model.message.clone()));
            }
            model.push_event(model.message.clone());
            refresh(app, model).await?;
        }
        return Ok(None);
    }
    if line == "legacy-appearance-form" {
        model.appearance_form = Some(appearance::AppearanceForm::new(model));
        model.error = None;
        return Ok(None);
    }
    if matches!(line, "save-appearance" | "apply-ghostty") {
        if let Some(form) = &model.appearance_form {
            model.font_vga = form.font_vga;
            model.font_size = form.font_size;
            model.motion = form.speed > 0;
            model.animation_speed = form.speed.max(1);
            save_session(app, model)?;
            if line == "apply-ghostty" {
                appearance::open_ghostty(app, model)?;
            }
            model.appearance_form = None;
            model.error = None;
            model.message = if line == "apply-ghostty" {
                "Ghostty opened with the selected font. Q closes this window.".into()
            } else {
                "Motion applied; font saved for the next Ghostty window.".into()
            };
            model.message_job = None;
        }
        return Ok(None);
    }
    if line == "export-form" {
        model.settings = None;
        let id = model
            .selected_job
            .clone()
            .ok_or_else(|| Error::Command("Select a job before exporting.".into()))?;
        let status = app.status(&id)?;
        if status.counts.open() > 0 {
            return Err(Error::NotTerminal {
                open: status.counts.open(),
            });
        }
        if model.working {
            return Err(Error::Msg(
                "Wait for the local write before exporting.".into(),
            ));
        }
        model.export_form = Some(forms::ExportForm {
            job: id,
            format_csv: false,
            partial: false,
            focus: 0,
        });
        model.error = None;
        return Ok(None);
    }
    if line == "save-export" {
        if model.working || model.pending_work.is_some() {
            return Err(Error::LocalBusy);
        }
        if let Some(form) = &model.export_form {
            let unknown = app.status(&form.job)?.counts.unknown;
            if unknown > 0 && !form.partial {
                return Err(Error::UnknownBlocks { unknown });
            }
            model.pending_work = Some(work::Work::Export {
                id: form.job.clone(),
                format: if form.format_csv {
                    crate::engine::OutFormat::Csv
                } else {
                    crate::engine::OutFormat::Parquet
                },
                partial: form.partial,
            });
        }
        model.export_form = None;
        return Ok(None);
    }

    if line == "home-view" {
        model.settings = None;
        model.home = true;
        model.command_mode = false;
        model.error = None;
        return Ok(None);
    }
    if line == "jobs-view" {
        model.settings = None;
        model.home = false;
        model.panel = 0;
        model.scroll = 0;
        refresh(app, model).await?;
        return Ok(None);
    }
    if line == "events-view" {
        model.home = false;
        model.panel = 3;
        model.scroll = 0;
        model.help = false;
        return Ok(None);
    }
    if line == "motion-toggle" {
        model.motion = !model.motion;
        model.message = format!(
            "Motion {}. Percentages reflect processed records.",
            if model.motion { "on" } else { "off" }
        );
        return Ok(None);
    }
    if matches!(line, "threads-form" | "default-threads-form") {
        model.settings = None;
        let job = if line == "default-threads-form" {
            None
        } else {
            Some(model.selected_job.clone().ok_or_else(|| {
                Error::Command("N: create a job; Settings > Processing sets defaults.".into())
            })?)
        };
        let previous = if let Some(id) = &job {
            if app.status(id)?.state == "RUNNING"
                || (model.busy && model.active_job.as_ref().is_none_or(|active| active == id))
            {
                return Err(Error::ThreadsBusy);
            }
            app.job_config(id)?.concurrency
        } else {
            app.thread_defaults()
                .map(|settings| settings.threads)
                .unwrap_or(8)
        };
        model.thread_form = Some(forms::ThreadForm {
            job,
            value: previous.to_string(),
            previous,
            cursor: None,
        });
        model.form = None;
        model.import_form = None;
        model.error = None;
        model.help = false;
        return Ok(None);
    }
    if line == "save-threads" {
        if let Some(form) = &model.thread_form {
            let threads = form.validate()?;
            if let Some(id) = &form.job {
                app.configure_threads(id, threads)?;
            } else {
                app.set_default_threads(threads)?;
            }
            model.message = format!(
                "{threads} threads saved for {}.",
                if form.job.is_some() {
                    "the selected job; RPS preserved"
                } else {
                    "new jobs; existing jobs preserved"
                }
            );
            model.push_event(model.message.clone());
        }
        model.thread_form = None;
        model.error = None;
        refresh(app, model).await?;
        return Ok(None);
    }
    if line == "import-form" {
        model.settings = None;
        if model.working || model.pending_work.is_some() {
            return Err(Error::LocalBusy);
        }
        model.import_form = Some(model.import_draft.take().unwrap_or(
            forms::ImportForm::from_defaults(&app.processing_defaults()?),
        ));
        model.form = None;
        model.thread_form = None;
        model.error = None;
        model.help = false;
        return Ok(None);
    }
    if line == "save-import" {
        if model.working || model.pending_work.is_some() {
            return Err(Error::LocalBusy);
        }
        if let Some(form) = &model.import_form {
            let (path, config) = form.config(app)?;
            model.import_draft = Some(form.clone());
            model.pending_work = Some(work::Work::Import(path, Box::new(config)));
        }
        model.import_form = None;
        model.error = None;
        return Ok(None);
    }
    if line == "save-api" {
        if let Some(form) = &model.form {
            form.save(app)?;
        }
        model.form = None;
        model.error = None;
        model.message = "Settings saved; secret protected in Keychain".into();
        model.push_event(model.message.clone());
        refresh(app, model).await?;
        return Ok(None);
    }
    if line == "legacy-configure" {
        if model.busy {
            return Err(Error::Msg(
                "Pause and wait before changing credentials".into(),
            ));
        }
        model.form = Some(ConfigForm::new(app.api_config().unwrap_or_default()));
        model.import_form = None;
        model.thread_form = None;
        model.error = None;
        model.help = false;
        model.message = "Enter a secret in Settings > Secrets".into();
        return Ok(None);
    }
    if line == "help" {
        model.help = true;
        model.scroll = 0;
        model.help_page = 0;
        return Ok(None);
    }
    let cli = parse_line(line)?;
    let Some(cmd) = cli.command else {
        return Ok(None);
    };
    if matches!(cmd, Cmd::Tui) {
        model.message = "The interface is already open".into();
        return Ok(None);
    }
    if let Cmd::Auth {
        action:
            AuthCmd::Configure {
                client_id,
                endpoint,
                token_url,
                allow_remote,
                disable_remote,
            },
    } = cmd
    {
        if model.busy {
            return Err(Error::Msg(
                "Pause and wait before changing credentials".into(),
            ));
        }
        let mut config = app.api_config()?;
        if let Some(id) = client_id {
            config.client_id = id;
        }
        if let Some(url) = endpoint {
            config.process_url = url;
        }
        if let Some(url) = token_url {
            config.token_url = url;
        }
        if allow_remote {
            config.remote_enabled = true;
        }
        if disable_remote {
            config.remote_enabled = false;
        }
        let mut settings = settings::Settings::new(app, model, 0)?;
        settings.api = ConfigForm::new(config);
        model.settings = Some(settings);
        model.import_form = None;
        model.thread_form = None;
        model.error = None;
        model.message = "Enter a secret in Settings > Secrets".into();
        return Ok(None);
    }
    if let Cmd::Job {
        action: JobCmd::Run { id } | JobCmd::Resume { id },
    } = cmd
    {
        if model.busy || model.working {
            return Err(Error::Locked);
        }
        model.selected_job = Some(app.status(&id)?.id);
        model.home = false;
        model.panel = 1;
        return Ok(Some(id));
    }
    if let Cmd::Job { action } = cmd {
        let work = match action {
            JobCmd::Create {
                input,
                endpoint,
                token_url,
                test_mode,
                contract,
                batch_size,
                rps,
                concurrency,
                validation,
                idempotency,
                client_id,
            } => {
                let config = crate::cli::config_from_flags(
                    endpoint,
                    token_url,
                    test_mode,
                    contract,
                    batch_size,
                    rps,
                    concurrency,
                    validation,
                    idempotency,
                    client_id,
                    app,
                )?;
                Some(work::Work::Import(input, Box::new(config)))
            }
            JobCmd::Checksum { id } => Some(work::Work::Checksum(id)),
            JobCmd::Merge {
                id,
                format,
                allow_unknown,
            } => Some(work::Work::Export {
                id,
                format: match format {
                    crate::cli::FormatArg::Csv => crate::engine::OutFormat::Csv,
                    _ => crate::engine::OutFormat::Parquet,
                },
                partial: allow_unknown,
            }),
            JobCmd::Export { id, format } => Some(work::Work::Export {
                id,
                format: match format {
                    crate::cli::FormatArg::Csv => crate::engine::OutFormat::Csv,
                    _ => crate::engine::OutFormat::Parquet,
                },
                partial: false,
            }),
            JobCmd::Prepare { id } => Some(work::Work::Prepare(id)),
            other => {
                let list = matches!(other, JobCmd::List);
                let status = matches!(other, JobCmd::Status { .. });
                let pause = matches!(other, JobCmd::Pause { .. });
                let (_, lines) = Box::pin(execute_collect(app, Cmd::Job { action: other })).await?;
                for line in lines {
                    model.push_event(line);
                }
                if list {
                    model.home = false;
                    model.panel = 0;
                }
                if status {
                    model.home = false;
                    model.panel = 1;
                }
                model.message = if pause {
                    "PAUSING · waiting for requests"
                } else {
                    "Done; Events shows the full result."
                }
                .into();
                model.error = None;
                refresh(app, model).await?;
                None
            }
        };
        if let Some(work) = work {
            if model.working || model.pending_work.is_some() {
                return Err(Error::LocalBusy);
            }
            model.pending_work = Some(work);
            model.error = None;
        }
        return Ok(None);
    }
    let diagnostic = matches!(&cmd, Cmd::Doctor);
    let (code, output) = Box::pin(execute_collect(app, cmd)).await?;
    model.home = false;
    model.panel = 3;
    model.scroll = 0;
    model.message_job = None;
    model.error = None;
    model.message = if diagnostic {
        "Diagnostics: review the results.".into()
    } else if code == 0 {
        "Command completed".into()
    } else {
        "Review items requiring attention.".into()
    };
    if diagnostic {
        model.push_event(output.join("\n"));
    } else {
        for message in output {
            model.push_event(message);
        }
    }
    refresh(app, model).await?;
    Ok(None)
}

#[cfg(test)]
mod commands_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ux_tests;

#[cfg(test)]
mod settings_tests;
