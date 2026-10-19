//! Interactive local matching workflow. One joined worker owns all disk/network work.
use super::{bad, Job, Phase, Progress, Rule, RunControl, Service, Settings, Snapshot};
use crate::{
    engine::App,
    error::{Error, Secret},
    tui::{self, appearance::AppearanceForm, forms, session, terminal::CrosstermTerm, Model},
};
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures_util::StreamExt;
use std::{
    io::{self, IsTerminal},
    path::PathBuf,
};
use tokio::{sync::watch, task::JoinHandle};
use zeroize::{Zeroize, Zeroizing};

mod view;
pub use view::render;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    NewJob,
    Jobs,
    Settings,
    Export,
    Threads,
    Delete,
    Help,
}
pub struct Ui {
    pub model: Model,
    pub page: Page,
    pub jobs: Vec<Job>,
    pub data_root: Option<PathBuf>,
    pub selected: usize,
    pub snapshot: Option<Snapshot>,
    snapshot_job: Option<String>,
    pub config: Settings,
    pub tab: usize,
    pub focus: Option<usize>,
    pub values: Vec<String>,
    pub cursor: Option<usize>,
    pub rule: Rule,
    appearance: AppearanceForm,
    pub message: String,
    pub failed: bool,
    pub command: Option<String>,
    pub completion: usize,
    pub busy: bool,
    pub progress: Option<Progress>,
    pub signed_in: bool,
    pub manual: super::manual::Pager,
    help_origin: Page,
}
impl Ui {
    pub fn new(config: Settings) -> Self {
        let mut model = Model::new();
        model.matching_mode = true;
        let appearance = AppearanceForm::new(&model);
        let rule = config.rule.clone();
        let mut ui = Self {
            model,
            page: Page::Home,
            jobs: Vec::new(),
            data_root: None,
            selected: 0,
            snapshot: None,
            snapshot_job: None,
            config,
            tab: 0,
            focus: None,
            values: Vec::new(),
            cursor: None,
            rule,
            appearance,
            message: String::new(),
            failed: false,
            command: None,
            completion: 0,
            busy: false,
            progress: None,
            signed_in: false,
            manual: super::manual::Pager::default(),
            help_origin: Page::Home,
        };
        ui.open(Page::Home);
        ui
    }
    fn notice(&mut self, text: impl Into<String>, failed: bool) {
        self.message = text.into();
        self.failed = failed;
    }
    pub fn open(&mut self, page: Page) {
        if page == Page::Help {
            if self.page != Page::Help {
                self.help_origin = self.page;
                self.manual = super::manual::Pager::default();
                self.manual.chapter = match self.page {
                    Page::NewJob => 6,
                    Page::Jobs | Page::Export | Page::Threads | Page::Delete => 8,
                    Page::Settings => 9,
                    _ => 2,
                };
                self.page = Page::Help;
                self.command = None;
            }
            return;
        }
        let page = if !self.signed_in
            && matches!(
                page,
                Page::NewJob | Page::Jobs | Page::Export | Page::Threads | Page::Delete
            ) {
            Page::Home
        } else {
            page
        };
        self.values.iter_mut().for_each(Zeroize::zeroize);
        self.message.clear();
        self.failed = false;
        self.page = page;
        self.cursor = None;
        self.command = None;
        self.completion = 0;
        match page {
            Page::Home if !self.signed_in => {
                self.values = vec![
                    self.config.client_id.clone(),
                    String::new(),
                    self.config.token_url.clone(),
                    String::new(),
                ];
                self.focus = Some(0);
            }
            Page::NewJob => {
                self.rule = self.config.rule.clone();
                self.values = vec![
                    String::new(),
                    self.rule.column.clone(),
                    self.config.threads.to_string(),
                ];
                self.focus = Some(0);
            }
            Page::Settings => {
                self.tab = 0;
                self.focus = None;
                self.appearance = AppearanceForm::new(&self.model);
                self.settings_values();
            }
            Page::Export => {
                self.values = vec![self
                    .current()
                    .ok()
                    .and_then(|job| super::cli::export_target(&job.id, None).ok())
                    .map_or_else(|| "./".into(), |path| path.display().to_string())];
                self.focus = Some(0);
            }
            Page::Threads => {
                self.values = vec![self
                    .jobs
                    .get(self.selected)
                    .map_or(self.config.threads, |j| j.threads)
                    .to_string()];
                self.focus = Some(0);
            }
            Page::Delete => {
                self.values.clear();
                self.focus = Some(0);
            }
            _ => self.focus = None,
        }
    }
    fn settings_values(&mut self) {
        self.cursor = None;
        self.values = match self.tab {
            0 => vec![
                self.config.url.clone(),
                self.config.scope.clone(),
                self.config.max_download_mib.to_string(),
                self.config.max_memory_mib.to_string(),
                self.config.timeout_seconds.to_string(),
                self.config.download_timeout_seconds.to_string(),
                self.config.download_idle_seconds.to_string(),
                self.config.download_part_mib.to_string(),
                self.config.download_parallelism.to_string(),
                serde_json::to_string(&self.config.download_form).unwrap_or_default(),
                self.config.download_retries.to_string(),
            ],
            1 => vec![
                self.config.rule.column.clone(),
                if self.config.rule.delimiter == b'\t' {
                    "tab".into()
                } else {
                    (self.config.rule.delimiter as char).to_string()
                },
                self.config.checkpoint_rows.to_string(),
                self.config.threads.to_string(),
            ],
            _ => Vec::new(),
        };
    }
    fn commit_draft(&mut self) -> Result<(), Error> {
        match self.tab {
            0 => {
                self.config.url = self.values[0].clone();
                self.config.scope = self.values[1].clone();
                self.config.max_download_mib = number(&self.values[2])?;
                self.config.max_memory_mib = number(&self.values[3])?;
                self.config.timeout_seconds = number(&self.values[4])?;
                self.config.download_timeout_seconds = number(&self.values[5])?;
                self.config.download_idle_seconds = number(&self.values[6])?;
                self.config.download_part_mib = number(&self.values[7])?;
                self.config.download_parallelism = usize::try_from(number(&self.values[8])?)
                    .map_err(|_| bad("Download parallelism must be 1–16."))?;
                self.config.download_form =
                    serde_json::from_str(&self.values[9]).map_err(|_| {
                        bad("Multipart fields must be a JSON object of non-secret text values.")
                    })?;
                self.config.download_retries = u8::try_from(number(&self.values[10])?)
                    .map_err(|_| bad("GET retries must be 0–5."))?;
            }
            1 => {
                self.config.rule.column = self.values[0].clone();
                self.config.rule.delimiter = super::cli::parse_delimiter(&self.values[1])?;
                self.config.checkpoint_rows = number(&self.values[2])?;
                self.config.threads = thread_number(&self.values[3])?;
            }
            _ => {}
        }
        Ok(())
    }
    fn fields(&self) -> usize {
        match self.page {
            Page::Home if !self.signed_in => 5,
            Page::NewJob => 4,
            Page::Export | Page::Threads => 1,
            Page::Settings => [13, 7, 5, 5][self.tab],
            _ => 0,
        }
    }
    fn editable(&self) -> Option<usize> {
        let i = self.focus?;
        match self.page {
            Page::Home if !self.signed_in && i < 4 => Some(i),
            Page::NewJob if i < 2 => Some(i),
            Page::NewJob if i == 3 => Some(2),
            Page::Export | Page::Threads => Some(0),
            Page::Settings if self.tab == 0 => [
                Some(0),
                None,
                Some(5),
                Some(6),
                Some(2),
                Some(7),
                Some(8),
                Some(10),
                Some(9),
                Some(1),
                Some(4),
                Some(3),
                None,
            ]
            .get(i)
            .copied()
            .flatten(),
            Page::Settings if self.tab == 1 && i < 4 => Some(i),
            _ => None,
        }
    }
    fn paste(&mut self, text: &str) {
        if self.busy {
            return;
        }
        if let Some(i) = self.editable() {
            let limit = if self.page == Page::Home && !self.signed_in && i == 3 {
                64
            } else {
                4096
            };
            forms::paste(&mut self.values[i], &mut self.cursor, text, limit);
        }
        // Slash commands have no arbitrary arguments or credentials.
    }
    fn settings_key(&mut self, key: KeyEvent) -> Result<(), Error> {
        if self.focus.is_none() {
            match key.code {
                KeyCode::Left | KeyCode::Right => {
                    self.commit_draft()?;
                    self.values.iter_mut().for_each(Zeroize::zeroize);
                    self.tab = (self.tab + if key.code == KeyCode::Left { 3 } else { 1 }) % 4;
                    self.settings_values();
                }
                KeyCode::Tab | KeyCode::Down | KeyCode::Enter => self.focus = Some(0),
                _ => {}
            }
            return Ok(());
        }
        let i = self.focus.unwrap();
        if self.tab == 3 {
            self.appearance.focus = i;
            self.appearance.key(key);
        } else if let Some(i) = self.editable() {
            forms::edit(&mut self.values[i], &mut self.cursor, key, 4096);
        } else if matches!(
            key.code,
            KeyCode::Left | KeyCode::Right | KeyCode::Char(' ')
        ) {
            match (self.tab, i) {
                (0, 1) => self.config.download_mode = self.config.download_mode.next(),
                (0, 12) => self.config.allow_http_loopback = !self.config.allow_http_loopback,
                (1, 4) => self.config.rule.transform = self.config.rule.transform.next(),
                (1, 5) => self.config.rule.trim = !self.config.rule.trim,
                (1, 6) => self.config.rule.headers = !self.config.rule.headers,
                _ => {}
            }
        }
        match key.code {
            KeyCode::Tab | KeyCode::Down => {
                self.focus = if i + 1 == self.fields() {
                    None
                } else {
                    Some(i + 1)
                }
            }
            KeyCode::BackTab | KeyCode::Up => self.focus = i.checked_sub(1),
            _ => {}
        }
        if matches!(
            key.code,
            KeyCode::Tab | KeyCode::Down | KeyCode::BackTab | KeyCode::Up
        ) {
            self.cursor = None;
        }
        Ok(())
    }
    fn current(&self) -> Result<&Job, Error> {
        self.jobs
            .get(self.selected)
            .ok_or_else(|| bad("Create a job first: N opens New Job."))
    }
    fn require_login(&self) -> Result<(), Error> {
        if self.signed_in {
            Ok(())
        } else {
            Err(bad("Sign in on Home before continuing."))
        }
    }
    fn complete(
        &mut self,
        result: Result<Outcome, Error>,
        retry: Option<(Vec<String>, Rule)>,
    ) -> Option<String> {
        self.busy = false;
        self.progress = None;
        match result {
            Ok(Outcome::SignedIn(status)) => {
                self.signed_in = true;
                self.open(Page::Home);
                self.notice(
                    format!("Signed in · token expires in {} seconds", status.expires_in),
                    false,
                );
            }
            Ok(Outcome::Loaded(snapshot)) => {
                self.snapshot = Some(snapshot);
                self.snapshot_job = None;
                self.notice("DAT verified and loaded into memory.", false);
            }
            Ok(Outcome::Ran(job, snapshot)) => {
                self.snapshot_job = snapshot.as_ref().map(|_| job.id.clone());
                self.snapshot = snapshot;
                self.notice(
                    format!(
                        "{} · true {} · false {} · {} threads · DAT {}",
                        tui::view::state_label(&job.state),
                        job.positives,
                        job.negatives,
                        job.threads,
                        if job.state == "COMPLETED" {
                            "released"
                        } else {
                            "locked in RAM"
                        }
                    ),
                    false,
                );
                return Some(job.id.clone());
            }
            Ok(Outcome::JobFailed(id, error)) => {
                self.open(Page::Jobs);
                self.notice(format!("{error} · R retries this job."), true);
                return Some(id);
            }
            Ok(Outcome::Verified(ok)) => self.notice(
                if ok {
                    "SHA-256 MATCH"
                } else {
                    "SHA-256 MISMATCH"
                },
                !ok,
            ),
            Ok(Outcome::Exported(path)) => {
                self.notice(format!("Exported: {}", path.display()), false)
            }
            Ok(Outcome::Deleted) => {
                self.snapshot = None;
                self.snapshot_job = None;
                self.notice(
                    "Job deleted. Original input and external exports kept.",
                    false,
                );
            }
            Ok(Outcome::ThreadsChanged(job)) => {
                self.notice(format!("Matching threads: {}", job.threads), false);
                return Some(job.id.clone());
            }
            Ok(Outcome::Authenticated(status, refresh)) => self.notice(
                format!(
                    "Token {} · expires in {} seconds",
                    if refresh { "renewed" } else { "obtained" },
                    status.expires_in
                ),
                false,
            ),
            Err(error) => {
                if let Some((values, rule)) = retry {
                    self.values.iter_mut().for_each(Zeroize::zeroize);
                    self.values = values;
                    self.rule = rule;
                    self.page = Page::NewJob;
                    self.focus = Some(0);
                }
                if !self.signed_in && self.page == Page::Home {
                    self.focus = Some(1);
                    self.cursor = None;
                }
                self.notice(error.to_string(), true);
            }
        }
        None
    }
}
impl Drop for Ui {
    fn drop(&mut self) {
        self.values.iter_mut().for_each(Zeroize::zeroize);
    }
}
fn number(s: &str) -> Result<u64, Error> {
    s.parse().map_err(|_| bad("Enter a positive whole number."))
}
fn thread_number(s: &str) -> Result<u16, Error> {
    let value = s
        .parse()
        .map_err(|_| bad("Matching threads must be 1-256."))?;
    super::validate_threads(value)?;
    Ok(value)
}
pub const COMMANDS: [&str; 12] = [
    "/new",
    "/jobs",
    "/settings",
    "/download",
    "/run",
    "/pause",
    "/help",
    "/quit",
    "/token",
    "/refresh",
    "/delete",
    "/threads",
];
pub fn suggestions(input: &str) -> Vec<&'static str> {
    COMMANDS
        .into_iter()
        .filter(|s| s.starts_with(input))
        .collect()
}

enum Outcome {
    SignedIn(super::TokenStatus),
    Loaded(Snapshot),
    Ran(Box<Job>, Option<Snapshot>),
    JobFailed(String, Error),
    Deleted,
    ThreadsChanged(Box<Job>),
    Verified(bool),
    Exported(PathBuf),
    Authenticated(super::TokenStatus, bool),
}
struct Worker {
    handle: JoinHandle<Result<Outcome, Error>>,
    stop: watch::Sender<bool>,
    retry: Option<(Vec<String>, Rule)>,
    progress: Option<watch::Receiver<Progress>>,
}
impl Worker {
    fn cancel(&self) {
        let _ = self.stop.send(true);
    }
    fn refresh(&self, ui: &mut Ui) {
        if *self.stop.borrow() {
            ui.progress = None;
            ui.notice("Stopping safely…", false);
        } else if let Some(progress) = &self.progress {
            // Clone and release the watch borrow before rendering or awaiting.
            ui.progress = Some(progress.borrow().clone());
        }
    }
}

pub async fn launch(app: &App) -> Result<(), Error> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(Error::NotATty);
    }
    let service = app.matching()?;
    let mut ui = Ui::new(service.settings()?);
    ui.data_root = Some(std::path::absolute(&service.root)?);
    session::load_session(app, &mut ui.model)?;
    ui.appearance = AppearanceForm::new(&ui.model);
    ui.jobs = service.jobs()?;
    ui.selected = ui
        .model
        .selected_job
        .as_ref()
        .and_then(|id| ui.jobs.iter().position(|job| &job.id == id))
        .unwrap_or(0);
    // Downloads and jobs stay on joined workers; no DAT is restored from disk.
    let mut worker = None;
    let mut quitting = false;
    let mut guard = CrosstermTerm::enter()?;
    let backend = ratatui::backend::CrosstermBackend::new(io::stdout());
    let mut terminal = ratatui::Terminal::new(backend)?;
    let mut events = EventStream::new();
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(50));
    let result=async {
        loop {
            if worker.as_ref().is_some_and(|w:&Worker|w.handle.is_finished()) {
                let task=worker.take().unwrap();
                let finished_id=ui.complete(task.handle.await.unwrap_or_else(|_|Err(bad("Background worker interrupted."))),task.retry);
                let id=finished_id.or_else(||ui.jobs.get(ui.selected).map(|job|job.id.clone()));ui.jobs=service.jobs()?;
                ui.selected=id.and_then(|id|ui.jobs.iter().position(|job|job.id==id)).unwrap_or(0);
            }
            if quitting && worker.is_none(){break;}
            if let Some(worker)=&worker {worker.refresh(&mut ui);}
            ui.model.viewport=(terminal.size()?.width,terminal.size()?.height);
            terminal.draw(|frame|render(frame,&ui))?;
            tokio::select! {
                _=tick.tick()=>{
                    ui.model.animation=ui.model.animation.wrapping_add(1);
                    if ui.busy && ui.model.animation.is_multiple_of(10) {ui.jobs=service.jobs()?;}
                },
                event=events.next()=>match event {
                    Some(Ok(Event::Key(key))) if key.kind!=KeyEventKind::Release=>{
                        ui.model.touch();
                        match handle_key(app,&service,&mut ui,&mut worker,key).await {
                            Ok(true)=>{quitting=true;if let Some(worker)=&worker {worker.cancel();ui.notice("Stopping at a checkpoint…",false);}},
                            Ok(false)=>{},Err(error)=>ui.notice(error.to_string(),true),
                        }
                    },
                    Some(Ok(Event::Paste(mut text)))=>{ui.paste(&text);text.zeroize();},Some(Err(error))=>return Err(Error::Io(error)),None=>{quitting=true;if let Some(worker)=&worker{worker.cancel();}},_=>{},
                },
            }
        }
        ui.model.selected_job=ui.jobs.get(ui.selected).map(|job|job.id.clone());
        session::save_session(app,&ui.model)?;Ok(())
    }.await;
    // Never detach blocking matching/decoder tasks when terminal I/O fails.
    if let Some(worker) = worker {
        worker.cancel();
        let _ = worker.handle.await;
    }
    ui.snapshot = None;
    service.clear_filters()?;
    service.forget_access()?;
    guard.leave()?;
    result
}
async fn handle_key(
    app: &App,
    service: &Service,
    ui: &mut Ui,
    worker: &mut Option<Worker>,
    key: KeyEvent,
) -> Result<bool, Error> {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Ok(true);
    }
    if ui.page == Page::Help {
        if ui.manual.key(key, ui.model.viewport) {
            ui.page = ui.help_origin;
        }
        return Ok(false);
    }
    if key.code == KeyCode::F(1) {
        ui.open(Page::Help);
        return Ok(false);
    }
    if !ui.signed_in && ui.page == Page::Home {
        return login_key(service, ui, worker, key);
    }
    if let Some(command) = &mut ui.command {
        let matches = suggestions(command);
        match key.code {
            KeyCode::Esc => ui.command = None,
            KeyCode::Up => ui.completion = ui.completion.saturating_sub(1),
            KeyCode::Down => {
                ui.completion = (ui.completion + 1).min(matches.len().saturating_sub(1))
            }
            KeyCode::Tab => {
                if let Some(value) = matches.get(ui.completion) {
                    *command = (*value).into();
                    ui.completion = 0;
                }
            }
            KeyCode::Backspace => {
                command.pop();
                ui.completion = 0;
            }
            KeyCode::Char(c) if c.is_ascii_alphabetic() && command.len() < 16 => {
                command.push(c);
                ui.completion = 0;
            }
            KeyCode::Enter => {
                let value = matches
                    .get(ui.completion)
                    .copied()
                    .ok_or_else(|| bad("Choose a command from the list."))?;
                ui.command = None;
                return action(service, ui, worker, value).await;
            }
            _ => {}
        }
        return Ok(false);
    }
    if ui.page == Page::Delete {
        match key.code {
            KeyCode::Esc | KeyCode::Char('n' | 'N' | 'q') => ui.open(Page::Jobs),
            KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                ui.focus = Some(1 - ui.focus.unwrap_or(0))
            }
            KeyCode::Char('y' | 'Y') => delete_selected(service, ui, worker)?,
            KeyCode::Enter if ui.focus == Some(1) => delete_selected(service, ui, worker)?,
            KeyCode::Enter => ui.open(Page::Jobs),
            _ => {}
        }
        return Ok(false);
    }
    if matches!(
        ui.page,
        Page::NewJob | Page::Settings | Page::Export | Page::Threads
    ) {
        if key.code == KeyCode::Esc {
            ui.values.iter_mut().for_each(Zeroize::zeroize);
            ui.config = service.settings()?;
            ui.open(if ui.page == Page::Threads {
                Page::Jobs
            } else {
                Page::Home
            });
            return Ok(false);
        }
        if ui.page == Page::Settings {
            if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
                save_ui_settings(app, service, ui)?;
            } else if ui.tab == 2 && ui.focus == Some(4) && key.code == KeyCode::Enter {
                ui.require_login()?;
                ensure_idle(worker)?;
                start_auth(service, ui, worker, true)?;
            } else if ui.tab == 3 && ui.focus == Some(4) && key.code == KeyCode::Enter {
                ui.notice(
                    "Save with Ctrl+S; the Ghostty launcher applies the saved font profile.",
                    false,
                );
            } else {
                ui.settings_key(key)?;
            }
            return Ok(false);
        }
        if key.code == KeyCode::Enter {
            if ui.page == Page::Threads {
                ensure_idle(worker)?;
                let id = ui.current()?.id.clone();
                let threads = thread_number(&ui.values[0])?;
                let owned = service.clone();
                start(ui, worker, move |_| async move {
                    tokio::task::spawn_blocking(move || {
                        owned
                            .set_job_threads(&id, threads)
                            .map(|j| Outcome::ThreadsChanged(Box::new(j)))
                    })
                    .await
                    .map_err(|_| bad("Thread settings worker interrupted."))?
                });
                ui.page = Page::Jobs;
            } else if ui.page == Page::NewJob {
                ui.rule.column = ui.values[1].clone();
                ui.rule.validate()?;
                let path = PathBuf::from(ui.values[0].trim());
                if ui.values[0].trim().is_empty() {
                    return Err(bad("Enter a CSV or Parquet path."));
                }
                let threads = thread_number(&ui.values[2])?;
                start_job(service, ui, worker, Some((path, ui.rule.clone(), threads)))?;
                ui.page = Page::Jobs;
            } else {
                ensure_idle(worker)?;
                let job = ui.current()?;
                if job.state != "COMPLETED" {
                    return Err(bad("Complete the job before exporting."));
                }
                let id = job.id.clone();
                let requested = PathBuf::from(ui.values[0].trim());
                let owned = service.clone();
                start(ui, worker, move |_| async move {
                    tokio::task::spawn_blocking(move || {
                        if !owned.verify(&id)? {
                            return Err(bad("Job checksum mismatch."));
                        }
                        let target = super::cli::export_target(&id, Some(&requested))?;
                        super::cli::copy_result(&owned.result_path(&id)?, &target)?;
                        Ok(Outcome::Exported(target))
                    })
                    .await
                    .map_err(|_| bad("Export worker interrupted."))?
                });
                ui.page = Page::Jobs;
            }
            return Ok(false);
        }
        let focus = ui.focus.unwrap_or(0);
        match key.code {
            KeyCode::Tab | KeyCode::Down => {
                ui.focus = Some((focus + 1) % ui.fields());
                ui.cursor = None;
            }
            KeyCode::BackTab | KeyCode::Up => {
                ui.focus = Some((focus + ui.fields() - 1) % ui.fields());
                ui.cursor = None;
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Char(' ')
                if ui.page == Page::NewJob && focus == 2 =>
            {
                ui.rule.transform = ui.rule.transform.next()
            }
            _ => {
                if let Some(i) = ui.editable() {
                    forms::edit(&mut ui.values[i], &mut ui.cursor, key, 4096);
                }
            }
        }
        return Ok(false);
    }
    match key.code {
        KeyCode::Char('l' | 'L') => {
            ensure_idle(worker)?;
            service.forget_access()?;
            ui.signed_in = false;
            ui.snapshot = None;
            ui.snapshot_job = None;
            service.clear_filters()?;
            ui.open(Page::Home);
        }
        KeyCode::Char('/') => {
            ui.command = Some("/".into());
            ui.completion = 0;
        }
        KeyCode::Char('q' | 'Q') => return Ok(true),
        KeyCode::Char('n' | 'N') => ui.open(Page::NewJob),
        KeyCode::Char('j' | 'J') => ui.open(Page::Jobs),
        KeyCode::Char('s' | 'S') => ui.open(Page::Settings),
        KeyCode::Char('h' | 'H') | KeyCode::Esc => ui.open(Page::Home),
        KeyCode::Char('?') => ui.open(Page::Help),
        KeyCode::Char('d' | 'D') => {
            action(service, ui, worker, "/download").await?;
        }
        KeyCode::Char('r' | 'R') => start_job(service, ui, worker, None)?,
        KeyCode::Char('p' | 'P') => {
            if let Some(worker) = worker {
                worker.cancel();
                ui.notice("Pausing at a checkpoint…", false);
            }
        }
        KeyCode::Char('e' | 'E') if ui.page == Page::Jobs => {
            ui.current()?;
            ui.open(Page::Export);
        }
        KeyCode::Char('k' | 'K') if ui.page == Page::Jobs => {
            ensure_idle(worker)?;
            let id = ui.current()?.id.clone();
            let owned = service.clone();
            start(ui, worker, move |_| async move {
                tokio::task::spawn_blocking(move || owned.verify(&id).map(Outcome::Verified))
                    .await
                    .map_err(|_| bad("Checksum worker interrupted."))?
            });
        }
        KeyCode::Delete | KeyCode::Char('x' | 'X') if ui.page == Page::Jobs => {
            ensure_idle(worker)?;
            ui.current()?;
            ui.open(Page::Delete);
        }
        KeyCode::Char('t' | 'T') if ui.page == Page::Jobs => {
            ensure_idle(worker)?;
            ui.current()?;
            ui.open(Page::Threads);
        }
        KeyCode::Left | KeyCode::BackTab if ui.page == Page::Home => {
            ui.model.home_selection = (ui.model.home_selection + 2) % 3
        }
        KeyCode::Right | KeyCode::Tab if ui.page == Page::Home => {
            ui.model.home_selection = (ui.model.home_selection + 1) % 3
        }
        KeyCode::Enter if ui.page == Page::Home => {
            ui.open([Page::NewJob, Page::Jobs, Page::Settings][ui.model.home_selection])
        }
        KeyCode::Up if ui.page == Page::Jobs => ui.selected = ui.selected.saturating_sub(1),
        KeyCode::Down if ui.page == Page::Jobs => {
            ui.selected = (ui.selected + 1).min(ui.jobs.len().saturating_sub(1))
        }
        _ => {}
    }
    Ok(false)
}
fn save_ui_settings(app: &App, service: &Service, ui: &mut Ui) -> Result<(), Error> {
    if ui.busy {
        return Err(bad("Pause the active operation before saving settings."));
    }
    ui.commit_draft()?;
    let previous = service.settings()?;
    if previous.client_id != ui.config.client_id || previous.token_url != ui.config.token_url {
        return Err(bad(
            "Sign out to change client ID, client secret or token URL.",
        ));
    }
    let profile_changed = previous.url != ui.config.url
        || previous.token_url != ui.config.token_url
        || previous.client_id != ui.config.client_id
        || previous.scope != ui.config.scope
        || previous.oauth != ui.config.oauth
        || previous.bearer != ui.config.bearer
        || previous.download_mode != ui.config.download_mode
        || previous.download_form != ui.config.download_form
        || previous.allow_http_loopback != ui.config.allow_http_loopback;
    let require_relogin = ui.signed_in && profile_changed;
    if require_relogin {
        service.forget_access()?;
        ui.signed_in = false;
        ui.snapshot = None;
        ui.snapshot_job = None;
        service.clear_filters()?;
    }
    service.save_settings(&ui.config)?;
    if profile_changed || previous.max_memory_mib != ui.config.max_memory_mib {
        ui.snapshot = None;
    }
    ui.model.font_vga = ui.appearance.font_vga;
    ui.model.font_size = ui.appearance.font_size;
    ui.model.motion = ui.appearance.speed > 0;
    ui.model.animation_speed = ui.appearance.speed.max(1);
    ui.model.scene = ui.appearance.scene;
    session::save_session(app, &ui.model)?;
    if ui.tab == 3 {
        let profile = app.paths.root.join("ghostty-ansi.config");
        std::fs::write(&profile, crate::tui::appearance::profile(&ui.model))?;
        crate::error::restrict_file(&profile)?;
    }
    ui.notice("Settings saved.", false);
    if require_relogin {
        ui.open(Page::Home);
        ui.notice("Settings saved · sign in for the updated profile.", false);
    }
    Ok(())
}
fn login_key(
    service: &Service,
    ui: &mut Ui,
    worker: &mut Option<Worker>,
    key: KeyEvent,
) -> Result<bool, Error> {
    if ui.busy {
        if key.code == KeyCode::Esc {
            if let Some(worker) = worker {
                worker.cancel();
            }
        }
        return Ok(false);
    }
    let i = ui.focus.unwrap_or(0);
    match key.code {
        KeyCode::F(2) => ui.open(Page::Settings),
        KeyCode::F(1) => ui.open(Page::Help),
        KeyCode::Esc => return Ok(true),
        KeyCode::Tab | KeyCode::Down => {
            ui.focus = Some((i + 1) % 5);
            ui.cursor = None;
        }
        KeyCode::BackTab | KeyCode::Up => {
            ui.focus = Some((i + 4) % 5);
            ui.cursor = None;
        }
        KeyCode::Enter if i < 3 && !key.modifiers.contains(KeyModifiers::CONTROL) => {
            ui.focus = Some(i + 1);
            ui.cursor = None;
        }
        KeyCode::Enter => {
            ensure_idle(worker)?;
            let encoded_key = Zeroizing::new(std::mem::take(&mut ui.values[3]));
            let secret = Secret::new(std::mem::take(&mut ui.values[1])).map_err(|_| {
                ui.focus = Some(1);
                ui.cursor = None;
                bad("Enter your client secret.")
            })?;
            let dat_key = super::crypto::DatKey::from_base64(&encoded_key).inspect_err(|_| {
                ui.focus = Some(3);
                ui.cursor = None;
            })?;
            drop(encoded_key);
            let mut cfg = ui.config.clone();
            cfg.client_id = ui.values[0].trim().to_owned();
            cfg.token_url = ui.values[2].trim().to_owned();
            cfg.reset_token_url.clear();
            cfg.oauth = true;
            cfg.bearer = false;
            if cfg.client_id.is_empty() {
                ui.focus = Some(0);
                ui.cursor = None;
                return Err(bad("Enter your client ID."));
            }
            cfg.api_url(&cfg.token_url, "Get token").inspect_err(|_| {
                ui.focus = Some(2);
                ui.cursor = None;
            })?;
            cfg.validate()?;
            ui.config = cfg.clone();
            let (owned, progress) = service.observed();
            start(ui, worker, move |stop| async move {
                let prepare = owned.clone();
                let preparing_stop = stop.clone();
                // Keep persistence off the input loop and always join preparation.
                tokio::task::spawn_blocking(move || {
                    if *preparing_stop.borrow() {
                        return Err(bad("Sign in cancelled."));
                    }
                    prepare.save_settings(&cfg)
                })
                .await
                .map_err(|_| bad("Sign in preparation interrupted."))??;
                if *stop.borrow() {
                    return Err(bad("Sign in cancelled. Enter your secret to retry."));
                }
                owned.phase(Phase::RequestingToken, 0, None);
                let status = tokio::select! {result=owned.sign_in(secret)=>result?,_=super::source::cancelled(stop.clone())=>return Err(bad("Sign in cancelled. Enter your secrets to retry."))};
                if *stop.borrow() {
                    return Err(bad("Sign in cancelled. Enter your secrets to retry."));
                }
                owned.set_dat_key(dat_key)?;
                Ok(Outcome::SignedIn(status))
            });
            worker.as_mut().unwrap().progress = Some(progress);
            ui.notice("Signing in…", false);
        }
        _ => {
            if let Some(i) = ui.editable() {
                let limit = if i == 3 { 64 } else { 4096 };
                forms::edit(&mut ui.values[i], &mut ui.cursor, key, limit);
            }
        }
    }
    Ok(false)
}
fn start_auth(
    service: &Service,
    ui: &mut Ui,
    worker: &mut Option<Worker>,
    refresh: bool,
) -> Result<(), Error> {
    ui.require_login()?;
    ensure_idle(worker)?;
    let (owned, progress) = service.observed();
    start(ui, worker, move |stop| async move {
        owned.phase(Phase::RequestingToken, 0, None);
        tokio::select! {result=async {if refresh{owned.refresh_token().await}else{owned.obtain_token().await}}=>result.map(|status|Outcome::Authenticated(status,refresh)),_=super::source::cancelled(stop)=>Err(bad("Authentication cancelled."))}
    });
    worker.as_mut().unwrap().progress = Some(progress);
    ui.notice(
        if refresh {
            "Renewing access token…"
        } else {
            "Obtaining access token…"
        },
        false,
    );
    Ok(())
}
async fn action(
    service: &Service,
    ui: &mut Ui,
    worker: &mut Option<Worker>,
    command: &str,
) -> Result<bool, Error> {
    if !matches!(command, "/settings" | "/help" | "/quit" | "/pause") {
        ui.require_login()?;
    }
    match command {
        "/new" => ui.open(Page::NewJob),
        "/jobs" => ui.open(Page::Jobs),
        "/settings" => ui.open(Page::Settings),
        "/help" => ui.open(Page::Help),
        "/delete" => {
            ensure_idle(worker)?;
            ui.current()?;
            ui.open(Page::Delete);
        }
        "/threads" => {
            ensure_idle(worker)?;
            ui.current()?;
            ui.open(Page::Threads);
        }
        "/quit" => return Ok(true),
        "/token" => start_auth(service, ui, worker, false)?,
        "/refresh" => start_auth(service, ui, worker, true)?,
        "/run" => start_job(service, ui, worker, None)?,
        "/pause" => {
            if let Some(worker) = worker {
                worker.cancel();
            }
        }
        "/download" => {
            ensure_idle(worker)?;
            let (owned, progress) = service.observed();
            start(ui, worker, move |stop| async move {
                owned.download(stop).await.map(Outcome::Loaded)
            });
            worker.as_mut().unwrap().progress = Some(progress);
        }
        _ => {}
    }
    Ok(false)
}
fn ensure_idle(worker: &Option<Worker>) -> Result<(), Error> {
    if worker.is_some() {
        Err(bad("An operation is active. P pauses it."))
    } else {
        Ok(())
    }
}
fn start<F, Fut>(ui: &mut Ui, worker: &mut Option<Worker>, task: F)
where
    F: FnOnce(watch::Receiver<bool>) -> Fut,
    Fut: std::future::Future<Output = Result<Outcome, Error>> + Send + 'static,
{
    let (stop, rx) = watch::channel(false);
    *worker = Some(Worker {
        handle: tokio::spawn(task(rx)),
        stop,
        retry: None,
        progress: None,
    });
    ui.busy = true;
    ui.progress = None;
    ui.notice("Starting operation… · P pause", false);
}
fn delete_selected(
    service: &Service,
    ui: &mut Ui,
    worker: &mut Option<Worker>,
) -> Result<(), Error> {
    ensure_idle(worker)?;
    let id = ui.current()?.id.clone();
    let owned = service.clone();
    if ui.snapshot_job.as_deref() == Some(&id) {
        ui.snapshot = None;
        ui.snapshot_job = None;
    }
    start(ui, worker, move |_| async move {
        tokio::task::spawn_blocking(move || {
            owned.delete_job(&id)?;
            Ok(Outcome::Deleted)
        })
        .await
        .map_err(|_| bad("Deletion worker interrupted."))?
    });
    ui.page = Page::Jobs;
    Ok(())
}
fn start_job(
    service: &Service,
    ui: &mut Ui,
    worker: &mut Option<Worker>,
    new: Option<(PathBuf, Rule, u16)>,
) -> Result<(), Error> {
    ui.require_login()?;
    ensure_idle(worker)?;
    if new.is_some() && ui.snapshot.is_none() && ui.config.url.is_empty() {
        return Err(bad("Configure the DAT source in Settings before starting."));
    }
    let retry = new
        .as_ref()
        .map(|(_, rule, _)| (ui.values.clone(), rule.clone()));
    let id = if new.is_none() {
        if ui.current()?.state == "COMPLETED" {
            return Err(bad(
                "Job complete. E exports results; K verifies checksums.",
            ));
        }
        Some(ui.current()?.id.clone())
    } else {
        None
    };
    let (owned, progress) = service.observed();
    let eligible = ui.snapshot_job.is_none() || ui.snapshot_job == id;
    let snapshot = if eligible {
        ui.snapshot.take()
    } else {
        ui.snapshot = None;
        None
    };
    ui.snapshot_job = None;
    start(ui, worker, move |stop| async move {
        let (id, threads) = if let Some((path, rule, threads)) = new {
            let service = owned.clone();
            let stop_import = stop.clone();
            let prepared = tokio::task::spawn_blocking(move || {
                service.create_job_cancellable_to(
                    &path,
                    rule,
                    &stop_import,
                    std::path::Path::new("."),
                )
            })
            .await
            .map_err(|_| bad("Import worker interrupted."))?;
            match prepared {
                Ok(job) => (job.id, Some(threads)),
                Err(error) => {
                    owned.discard_pending()?;
                    return Err(error);
                }
            }
        } else {
            (id.unwrap(), None)
        };
        let result = async {
            if let Some(threads) = threads {
                owned.set_job_threads(&id, threads)?;
            }
            if *stop.borrow() {
                return Err(bad("Import stopped; the prepared job can be run later."));
            }
            let job = owned
                .run_job(&id, snapshot, RunControl::with_stop(stop))
                .await?;
            let paused = if job.state == "PAUSED" {
                owned.job_filter(&id)?
            } else {
                None
            };
            Ok(Outcome::Ran(Box::new(job), paused))
        }
        .await;
        match result {
            Ok(outcome) => Ok(outcome),
            Err(error) => {
                let error = owned.discard_pending().err().unwrap_or(error);
                Ok(Outcome::JobFailed(id, error))
            }
        }
    });
    if let Some(worker) = worker {
        worker.retry = retry;
        worker.progress = Some(progress);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn waiting_retry_download_and_matching_are_readable_before_a_job_exists() {
        let mut ui = Ui::new(Settings::default());
        ui.signed_in = true;
        ui.open(Page::Jobs);
        ui.busy = true;
        for (width, height) in [(40, 18), (60, 22), (80, 24), (120, 36)] {
            for phase in [
                Phase::WaitingForDat,
                Phase::RetryWait,
                Phase::DownloadingDat,
                Phase::Matching,
            ] {
                let mut progress = Progress::new(phase);
                progress.retry = 1;
                progress.retry_limit = 2;
                progress.last_http = Some(504);
                progress.retry_http = Some(504);
                progress.completed = 10_000_000;
                progress.total = Some(20_000_000);
                ui.progress = Some(progress);
                let screen = text(&ui, width, height);
                assert!(screen.contains("LIVE ACTIVITY"), "{screen}");
                match phase {
                    Phase::WaitingForDat => {
                        assert!(screen.contains("Waiting for DAT"));
                        assert!(screen.contains("Last HTTP 504"));
                        assert!(screen.contains("2/3"));
                    }
                    Phase::RetryWait => assert!(screen.contains("HTTP 504 · retry 1/2")),
                    Phase::DownloadingDat => assert!(screen.contains("9.5/19.1 MiB")),
                    _ => assert!(screen.contains("Matching · 10000000/20000000")),
                }
                assert!(screen.contains("P pause"));
                assert!(!screen.contains("Working locally"));
            }
        }
        let root = tempfile::tempdir().unwrap();
        let service = Service::open(
            root.path().join("matching"),
            std::sync::Arc::new(crate::secrets::MemorySecrets::new()),
        )
        .unwrap();
        let input = root.path().join("dataset.csv");
        std::fs::write(&input, "identifier\nexample\n").unwrap();
        ui.jobs
            .push(service.create_job(&input, Rule::default()).unwrap());
        for (width, height) in [(40, 18), (60, 22), (80, 24), (120, 36)] {
            let screen = text(&ui, width, height);
            assert!(screen.contains("dataset.csv"), "{screen}");
            assert!(screen.contains("LIVE ACTIVITY"));
        }
    }

    #[tokio::test]
    async fn stopping_and_completion_clear_live_progress_instead_of_overwriting_the_stop_notice() {
        let root = tempfile::tempdir().unwrap();
        let service = Service::open(
            root.path().into(),
            std::sync::Arc::new(crate::secrets::MemorySecrets::new()),
        )
        .unwrap();
        let (_, rx) = service.observed();
        let mut ui = Ui::new(Settings::default());
        let mut worker = None;
        start(&mut ui, &mut worker, |stop| async move {
            super::super::source::cancelled(stop).await;
            Err(bad("Stopped."))
        });
        let worker = worker.as_mut().unwrap();
        worker.progress = Some(rx);
        worker.refresh(&mut ui);
        assert!(ui.progress.is_some());
        worker.cancel();
        worker.refresh(&mut ui);
        assert!(ui.progress.is_none());
        assert_eq!(ui.message, "Stopping safely…");
        ui.progress = Some(Progress::new(Phase::Matching));
        ui.complete(Err(bad("Stopped.")), None);
        assert!(!ui.busy);
        assert!(ui.progress.is_none());
        // The worker exits through the same cancellation receiver.
        while !worker.handle.is_finished() {
            tokio::task::yield_now().await;
        }
    }
    fn test_dat_key() -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode([0x42; 32])
    }
    fn text(ui: &Ui, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| render(f, ui)).unwrap();
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    #[test]
    fn screens_render_at_four_sizes_and_preserve_simple_english_flow() {
        let mut ui = Ui::new(Settings::default());
        ui.signed_in = true;
        for (width, height) in [(40, 18), (60, 22), (80, 24), (120, 36)] {
            for page in [
                Page::Home,
                Page::NewJob,
                Page::Jobs,
                Page::Settings,
                Page::Export,
                Page::Threads,
                Page::Delete,
                Page::Help,
            ] {
                ui.open(page);
                let screen = text(&ui, width, height);
                assert!(screen.contains("MATCHPOINT"));
                if page != Page::Help {
                    assert!(screen.contains("New Job"));
                    assert!(!screen.contains("CPF"));
                }
            }
        }
        ui.open(Page::Home);
        let screen = text(&ui, 80, 24);
        assert!(screen.contains("Q quit"));
        assert!(screen.contains("L sign out"));
        assert!(screen.contains("MATCH DISTRIBUTION"));
        ui.open(Page::NewJob);
        let screen = text(&ui, 80, 24);
        assert!(screen.contains("CSV or Parquet"));
        assert!(screen.contains("Key column"));
        assert!(screen.contains("Transformation"));
    }
    #[test]
    fn authenticated_settings_have_no_credential_editors() {
        let mut ui = Ui::new(Settings {
            client_id: "original-client".into(),
            token_url: "https://auth.example.test/token".into(),
            ..Default::default()
        });
        ui.signed_in = true;
        ui.open(Page::Settings);
        for tab in 0..3 {
            ui.tab = tab;
            ui.settings_values();
            for i in 0..ui.fields() {
                ui.focus = Some(i);
                if tab == 2 {
                    assert!(ui.editable().is_none());
                    ui.paste("forbidden-secret-replacement");
                    ui.settings_key(KeyCode::Char('x').into()).unwrap();
                    assert!(ui.values.is_empty());
                }
                for (w, h) in [(40, 18), (80, 24), (120, 36)] {
                    let screen = text(&ui, w, h);
                    assert!(!screen.contains("Client secret"));
                    assert!(!screen.contains("OAuth client ID"));
                    assert!(!screen.contains("Reset token URL"));
                    assert!(!screen.contains("forbidden-secret"));
                }
            }
        }
        assert_eq!(ui.config.client_id, "original-client");
    }
    #[tokio::test]
    async fn manual_keys_are_inert_and_restore_login_and_job_drafts() {
        let tmp = tempfile::tempdir().unwrap();
        let app = login_app(tmp.path());
        let service = app.matching().unwrap();
        let mut ui = Ui::new(Settings::default());
        let mut worker = None;
        for page in [Page::Home, Page::NewJob, Page::Settings] {
            ui.signed_in = page != Page::Home;
            ui.open(page);
            ui.values[0] = "draft-kept-locally".into();
            ui.focus = Some(0);
            let event = |code| KeyEvent::new(code, KeyModifiers::NONE);
            assert_eq!(text(&ui, 80, 24).matches("MATCHPOINT").count(), 1);
            handle_key(&app, &service, &mut ui, &mut worker, event(KeyCode::F(1)))
                .await
                .unwrap();
            assert!(!text(&ui, 80, 24).contains("draft-kept-locally"));
            for code in [
                KeyCode::Char('/'),
                KeyCode::Char('r'),
                KeyCode::Enter,
                KeyCode::Char('n'),
                KeyCode::Char('q'),
            ] {
                assert!(
                    !handle_key(&app, &service, &mut ui, &mut worker, event(code))
                        .await
                        .unwrap()
                );
            }
            assert!(ui.page == page);
            assert_eq!(ui.values[0], "draft-kept-locally");
            assert!(worker.is_none());
        }
        assert!(service.jobs().unwrap().is_empty());
    }
    #[test]
    fn api_fields_remain_editable_and_visible_when_scrolled() {
        let mut ui = Ui::new(Settings::default());
        ui.signed_in = true;
        ui.open(Page::Settings);
        ui.focus = Some(0);
        ui.paste("https://data.example.test/filter.dat");
        for (w, h) in [(40, 18), (80, 24), (120, 36)] {
            assert!(text(&ui, w, h).contains("DAT URL"));
        }
        ui.commit_draft().unwrap();
        assert!(ui.config.url.ends_with("filter.dat"));
        assert!(ui.config.client_id.is_empty());
        assert!(ui.config.token_url.is_empty());
    }
    #[tokio::test]
    async fn session_button_renews_at_the_login_endpoint_without_credential_editors() {
        let tmp = tempfile::tempdir().unwrap();
        let fixture = crate::sim::Fixture::spawn().await;
        let app = login_app(tmp.path());
        let service = app.matching().unwrap();
        let cfg = Settings {
            url: fixture.process_url(),
            token_url: fixture.token_url(),
            client_id: "fixture".into(),
            allow_http_loopback: true,
            ..Default::default()
        };
        service.save_settings(&cfg).unwrap();
        let mut ui = Ui::new(cfg);
        ui.values[1] = "synthetic-client-secret-hidden".into();
        ui.values[3] = test_dat_key();
        ui.focus = Some(3);
        let mut worker = None;
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
            .await
            .unwrap();
        finish(&mut ui, &mut worker).await;
        assert!(ui.signed_in);
        ui.open(Page::Settings);
        ui.tab = 2;
        ui.settings_values();
        ui.focus = Some(4);
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
            .await
            .unwrap();
        let task = worker.take().unwrap();
        let outcome = task.handle.await.unwrap().unwrap();
        assert!(matches!(outcome, Outcome::Authenticated(_, true)));
        ui.complete(Ok(outcome), task.retry);
        assert!(ui.signed_in);
        assert!(ui.values.is_empty());
        assert_eq!(
            fixture.hits_token.load(std::sync::atomic::Ordering::SeqCst),
            2
        );
        assert!(!text(&ui, 80, 24).contains("sim-token"));
    }
    #[tokio::test]
    async fn tabs_drafts_save_cancel_navigation_and_slash_completion() {
        let tmp = tempfile::tempdir().unwrap();
        let app = App::open(
            tmp.path().into(),
            std::sync::Arc::new(crate::secrets::MemorySecrets::new()),
            std::sync::Arc::new(crate::timeutil::SystemClock),
        )
        .unwrap();
        let service = app.matching().unwrap();
        let mut ui = Ui::new(Settings::default());
        ui.signed_in = true;
        let mut worker = None;
        handle_key(
            &app,
            &service,
            &mut ui,
            &mut worker,
            KeyCode::Char('s').into(),
        )
        .await
        .unwrap();
        ui.values[0] = "https://example.com/filter.dat".into();
        handle_key(
            &app,
            &service,
            &mut ui,
            &mut worker,
            KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
        )
        .await
        .unwrap();
        assert_eq!(
            service.settings().unwrap().url,
            "https://example.com/filter.dat"
        );
        assert!(!ui.signed_in);
        // Isolate the remaining authenticated editor/navigation checks.
        ui.signed_in = true;
        ui.open(Page::Settings);
        ui.values[0] = "https://example.com/discard".into();
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Esc.into())
            .await
            .unwrap();
        assert_eq!(ui.config.url, "https://example.com/filter.dat");
        for code in [
            KeyCode::Char('/'),
            KeyCode::Char('n'),
            KeyCode::Tab,
            KeyCode::Enter,
        ] {
            handle_key(&app, &service, &mut ui, &mut worker, code.into())
                .await
                .unwrap();
        }
        assert!(ui.page == Page::NewJob);
        assert_eq!(ui.focus, Some(0));
        ui.paste("/tmp/a path/data.parquet");
        assert_eq!(ui.values[0], "/tmp/a path/data.parquet");
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Tab.into())
            .await
            .unwrap();
        assert_eq!(ui.focus, Some(1));
        assert!(!suggestions("/s").is_empty());
        assert!(suggestions("/token=hidden").is_empty());
    }

    fn login_app(root: &std::path::Path) -> App {
        App::open(
            root.into(),
            std::sync::Arc::new(crate::secrets::MemorySecrets::new()),
            std::sync::Arc::new(crate::timeutil::SystemClock),
        )
        .unwrap()
    }
    async fn finish(ui: &mut Ui, worker: &mut Option<Worker>) {
        let task = worker.take().unwrap();
        let result = tokio::time::timeout(std::time::Duration::from_secs(3), task.handle)
            .await
            .unwrap()
            .unwrap();
        ui.complete(result, task.retry);
    }
    #[tokio::test]
    async fn completed_job_shows_its_real_absolute_result_and_export_default_at_every_size() {
        let root = tempfile::tempdir().unwrap();
        let app = login_app(root.path());
        let service = app.matching().unwrap();
        let input = root.path().join("keys.csv");
        std::fs::write(&input, "key\n0001\n").unwrap();
        let rule = Rule::default();
        let mut bloom = super::super::format::Bloom::new(1, &rule).unwrap();
        bloom.insert("0001").unwrap();
        let dat = root.path().join("fixture.dat");
        bloom.write(&dat).unwrap();
        let filter = Snapshot {
            bloom: std::sync::Arc::new(super::super::format::Bloom::read(&dat, 4096).unwrap()),
        };
        let job = service
            .create_job_cancellable_to(&input, rule, &RunControl::default().stop, root.path())
            .unwrap();
        service
            .run_job(&job.id, Some(filter), RunControl::default())
            .await
            .unwrap();
        let mut ui = Ui::new(service.settings().unwrap());
        ui.data_root = Some(std::path::absolute(&service.root).unwrap());
        ui.signed_in = true;
        ui.jobs = service.jobs().unwrap();
        ui.open(Page::Jobs);
        let expected = service.result_path(&job.id).unwrap().display().to_string();
        let compact = |value: &str| {
            value
                .chars()
                .filter(|c| !c.is_whitespace() && *c != '│')
                .collect::<String>()
        };
        for (w, h) in [(40, 18), (60, 22), (80, 24), (120, 36)] {
            let screen = text(&ui, w, h);
            assert!(
                screen.contains("RESULT") || compact(&screen).contains("Result:/"),
                "{screen}"
            );
            assert!(compact(&screen).contains(&compact(&expected)), "{screen}");
            assert!(!screen.contains("Result: matching/"));
        }
        ui.open(Page::Export);
        let target = PathBuf::from(&ui.values[0]);
        assert!(target.is_absolute());
        assert_eq!(target.parent().unwrap(), std::env::current_dir().unwrap());
        assert_eq!(
            target.file_name().unwrap(),
            format!("result-{}.txt", job.id).as_str()
        );
        for (w, h) in [(40, 18), (60, 22), (80, 24), (120, 36)] {
            let screen = text(&ui, w, h);
            assert!(screen.contains("Output path"));
            assert!(
                compact(&screen).contains(&compact(&target.display().to_string())),
                "{screen}"
            );
        }
        let output = root.path().join("export with spaces.txt");
        ui.values[0] = output.display().to_string();
        let mut worker = None;
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
            .await
            .unwrap();
        finish(&mut ui, &mut worker).await;
        assert_eq!(ui.message, format!("Exported: {}", output.display()));
        for (w, h) in [(40, 18), (60, 22), (80, 24), (120, 36)] {
            assert!(compact(&text(&ui, w, h)).contains(&compact(&output.display().to_string())));
        }
        assert_eq!(
            std::fs::read(output).unwrap(),
            std::fs::read(service.result_path(&job.id).unwrap()).unwrap()
        );
    }
    #[tokio::test]
    async fn failed_download_and_repeat_input_resume_one_prepared_job() {
        let tmp = tempfile::tempdir().unwrap();
        let app = login_app(tmp.path());
        let service = app.matching().unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let cfg = Settings {
            url: format!("http://{address}/filter"),
            allow_http_loopback: true,
            ..Default::default()
        };
        service.save_settings(&cfg).unwrap();
        let input = tmp.path().join("input.csv");
        std::fs::write(&input, "key\n000123\n").unwrap();
        let mut ui = Ui::new(cfg);
        ui.signed_in = true;
        ui.open(Page::NewJob);
        ui.values[0] = input.display().to_string();
        let mut worker = None;
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
            .await
            .unwrap();
        assert!(start_job(&service, &mut ui, &mut worker, None).is_err());
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
            .await
            .unwrap();
        let task = worker.take().unwrap();
        let outcome = task.handle.await.unwrap().unwrap();
        let id = ui.complete(Ok(outcome), task.retry).unwrap();
        assert!(ui.page == Page::Jobs && ui.failed);
        assert!(ui.message.contains("R retries this job"));
        ui.jobs = service.jobs().unwrap();
        assert_eq!(ui.jobs.len(), 1);
        assert_eq!(ui.jobs[0].id, id);
        assert_eq!(
            service.result_path(&id).unwrap(),
            std::env::current_dir()
                .unwrap()
                .join(format!("result-{id}.txt"))
        );
        for _ in 0..2 {
            handle_key(
                &app,
                &service,
                &mut ui,
                &mut worker,
                KeyCode::Char('r').into(),
            )
            .await
            .unwrap();
            finish(&mut ui, &mut worker).await;
            assert!(ui.page == Page::Jobs);
            assert_eq!(service.jobs().unwrap().len(), 1);
            assert_eq!(service.jobs().unwrap()[0].id, id);
        }
        ui.open(Page::NewJob);
        ui.values[0] = tmp.path().join("missing.csv").display().to_string();
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
            .await
            .unwrap();
        finish(&mut ui, &mut worker).await;
        assert!(ui.page == Page::NewJob && ui.failed);
        assert!(ui.values[0].contains("missing.csv"));
        assert_eq!(service.jobs().unwrap().len(), 1);
    }
    #[test]
    fn download_settings_modes_fields_and_limits_save_and_scroll_at_all_sizes() {
        let tmp = tempfile::tempdir().unwrap();
        let app = login_app(tmp.path());
        let service = app.matching().unwrap();
        let mut ui = Ui::new(Settings::default());
        ui.open(Page::Settings);
        ui.focus = Some(1);
        ui.settings_key(KeyCode::Right.into()).unwrap();
        assert_eq!(
            ui.config.download_mode,
            super::super::DownloadMode::Parallel
        );
        ui.settings_key(KeyCode::Char(' ').into()).unwrap();
        for (row, value) in [
            (2, "900"),
            (3, "45"),
            (5, "32"),
            (6, "8"),
            (7, "3"),
            (8, r#"{"format":"dat"}"#),
        ] {
            ui.focus = Some(row);
            let index = ui.editable().unwrap();
            ui.values[index].clear();
            ui.paste(value);
        }
        save_ui_settings(&app, &service, &mut ui).unwrap();
        let cfg = service.settings().unwrap();
        assert_eq!(cfg.download_mode, super::super::DownloadMode::Multipart);
        assert_eq!(
            (
                cfg.download_timeout_seconds,
                cfg.download_idle_seconds,
                cfg.download_part_mib,
                cfg.download_parallelism,
                cfg.download_retries
            ),
            (900, 45, 32, 8, 3)
        );
        assert_eq!(cfg.download_form["format"], "dat");
        for size in [(40, 18), (60, 22), (80, 24), (120, 36)] {
            ui.focus = Some(1);
            assert!(text(&ui, size.0, size.1).contains("Multipart POST"));
            ui.focus = Some(8);
            assert!(text(&ui, size.0, size.1).contains("Form fields"));
            ui.focus = Some(12);
            assert!(text(&ui, size.0, size.1).contains("Loopback HTTP"));
        }
    }
    #[tokio::test]
    async fn job_threads_and_delete_confirmation_work_at_every_supported_size() {
        let root = tempfile::tempdir().unwrap();
        let app = login_app(root.path());
        let service = app.matching().unwrap();
        let input = root.path().join("keys.csv");
        std::fs::write(&input, "identifier\n000123\n").unwrap();
        let job = service.create_job(&input, Rule::default()).unwrap();
        let mut ui = Ui::new(service.settings().unwrap());
        ui.signed_in = true;
        ui.jobs = service.jobs().unwrap();
        ui.open(Page::Jobs);
        let mut worker = None;
        handle_key(
            &app,
            &service,
            &mut ui,
            &mut worker,
            KeyCode::Char('t').into(),
        )
        .await
        .unwrap();
        assert!(ui.page == Page::Threads);
        ui.values[0] = "7".into();
        for (w, h) in [(40, 18), (60, 22), (80, 24), (120, 36)] {
            assert!(text(&ui, w, h).contains("MATCHING THREADS"));
        }
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
            .await
            .unwrap();
        finish(&mut ui, &mut worker).await;
        assert_eq!(service.job(&job.id).unwrap().threads, 7);
        handle_key(
            &app,
            &service,
            &mut ui,
            &mut worker,
            KeyCode::Char('x').into(),
        )
        .await
        .unwrap();
        assert!(ui.page == Page::Delete);
        for (w, h) in [(40, 18), (60, 22), (80, 24), (120, 36)] {
            let screen = text(&ui, w, h);
            assert!(screen.contains("Cannot be undone."));
            assert!(screen.contains("[ CANCEL ]"));
        }
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
            .await
            .unwrap();
        assert!(worker.is_none());
        assert!(service.job(&job.id).is_ok());
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Delete.into())
            .await
            .unwrap();
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Right.into())
            .await
            .unwrap();
        assert_eq!(ui.focus, Some(1));
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
            .await
            .unwrap();
        finish(&mut ui, &mut worker).await;
        assert!(service.jobs().unwrap().is_empty());
        assert!(input.exists());
    }
    #[test]
    fn home_starts_at_login_without_splash_and_masks_each_secret_at_all_sizes() {
        let mut ui = Ui::new(Settings::default());
        assert!(!ui.signed_in);
        assert!(!ui.model.splash);
        assert!(ui.page == Page::Home);
        ui.values[1] = "never-render-this-secret".into();
        ui.values[3] = test_dat_key();
        for (w, h) in [(40, 18), (60, 22), (80, 24), (120, 36)] {
            for (i, label) in ["Client ID", "Client secret", "Token URL", "DAT key"]
                .into_iter()
                .enumerate()
            {
                ui.focus = Some(i);
                let screen = text(&ui, w, h);
                assert!(screen.contains("SIGN IN"));
                assert!(screen.contains(label), "{w}x{h} {label}");
                assert!(screen.contains("Sign in →"));
                assert!(!screen.contains("never-render"));
                assert!(!screen.contains(&test_dat_key()));
                assert!(!screen.contains("MATCH DISTRIBUTION"));
            }
        }
        ui.open(Page::Settings);
        assert!(ui.values.iter().all(|s| !s.contains("never-render")));
    }
    #[tokio::test]
    async fn locked_navigation_commands_and_saved_credentials_cannot_skip_login() {
        let tmp = tempfile::tempdir().unwrap();
        let app = login_app(tmp.path());
        let service = app.matching().unwrap();
        let fixture = crate::sim::Fixture::spawn().await;
        let cfg = Settings {
            token_url: fixture.token_url(),
            client_id: "fixture".into(),
            oauth: true,
            allow_http_loopback: true,
            ..Settings::default()
        };
        service.save_settings(&cfg).unwrap();
        service
            .save_oauth_credentials(Some(&Secret::new("saved-secret".into()).unwrap()), None)
            .unwrap();
        service.obtain_token().await.unwrap();
        let mut ui = Ui::new(cfg);
        let mut worker = None;
        for page in [Page::NewJob, Page::Jobs, Page::Export] {
            ui.open(page);
            assert!(ui.page == Page::Home);
            assert!(!ui.signed_in);
        }
        for command in ["/new", "/jobs", "/download", "/run", "/token", "/refresh"] {
            assert!(action(&service, &mut ui, &mut worker, command)
                .await
                .is_err());
            assert!(worker.is_none());
        }
        ui.open(Page::Settings);
        ui.tab = 2;
        ui.settings_values();
        ui.focus = Some(4);
        assert!(
            handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
                .await
                .is_err()
        );
        assert!(worker.is_none());
        assert!(!ui.signed_in);
        assert_eq!(
            fixture.hits_token.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
    }
    #[tokio::test]
    async fn login_rejects_missing_fields_and_invalid_urls_without_network() {
        let tmp = tempfile::tempdir().unwrap();
        let app = login_app(tmp.path());
        let service = app.matching().unwrap();
        for (id, secret, url) in [
            ("", "secret", "https://example.test/token"),
            ("client", "", "https://example.test/token"),
            ("client", "secret", ""),
            ("client", "secret", "http://example.test/token"),
            (
                "client",
                "secret",
                "https://user:password@example.test/token",
            ),
        ] {
            let mut ui = Ui::new(Settings::default());
            ui.values = vec![id.into(), secret.into(), url.into(), test_dat_key()];
            ui.focus = Some(3);
            let mut worker = None;
            assert!(
                handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
                    .await
                    .is_err()
            );
            assert!(!ui.signed_in);
            assert!(worker.is_none());
            assert!(ui.values[1].is_empty());
            assert!(service.settings().unwrap().token_url.is_empty());
        }
    }
    #[tokio::test]
    async fn login_rejects_missing_or_invalid_dat_key_without_token_request_or_storage() {
        use base64::Engine;
        let root = tempfile::tempdir().unwrap();
        let app = login_app(root.path());
        let service = app.matching().unwrap();
        let fixture = crate::sim::Fixture::spawn().await;
        for encoded in [
            String::new(),
            "password-not-a-key".into(),
            "!".repeat(44),
            base64::engine::general_purpose::STANDARD.encode([0; 16]),
        ] {
            let mut ui = Ui::new(Settings {
                allow_http_loopback: true,
                ..Default::default()
            });
            ui.values = vec![
                "fixture".into(),
                "synthetic-client-secret".into(),
                fixture.token_url(),
                encoded,
            ];
            ui.focus = Some(3);
            let mut worker = None;
            assert!(
                handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
                    .await
                    .is_err()
            );
            assert!(!ui.signed_in);
            assert!(!ui.busy);
            assert!(worker.is_none());
            assert!(ui.values[1].is_empty());
            assert!(ui.values[3].is_empty());
            assert!(service.dat_key().unwrap().is_none());
        }
        assert_eq!(
            fixture.hits_token.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
        assert!(!service.root.join("settings.json").exists());
    }
    #[test]
    fn dat_key_paste_is_bounded_and_settings_never_expose_a_key_editor() {
        let mut ui = Ui::new(Settings::default());
        ui.focus = Some(3);
        ui.paste(&"x".repeat(4096));
        assert_eq!(ui.values[3].len(), 64);
        ui.signed_in = true;
        ui.open(Page::Settings);
        for tab in 0..4 {
            ui.tab = tab;
            ui.settings_values();
            assert!(ui.values.iter().all(|v| !v.contains(&"x".repeat(44))));
        }
    }
    #[tokio::test]
    async fn login_with_four_fields_unlocks_and_sign_out_requires_fresh_secrets() {
        let tmp = tempfile::tempdir().unwrap();
        let app = login_app(tmp.path());
        let service = app.matching().unwrap();
        let fixture = crate::sim::Fixture::spawn().await;
        let mut ui = Ui::new(Settings {
            allow_http_loopback: true,
            ..Settings::default()
        });
        let mut worker = None;
        ui.paste("fixture");
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
            .await
            .unwrap();
        ui.paste("fresh-login-secret");
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
            .await
            .unwrap();
        ui.paste(&fixture.token_url());
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
            .await
            .unwrap();
        assert_eq!(ui.focus, Some(3));
        ui.paste(&test_dat_key());
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
            .await
            .unwrap();
        assert!(!ui.signed_in);
        assert!(ui.busy);
        assert!(ui.values[1].is_empty());
        ui.paste("must-not-enter-busy-form");
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::F(2).into())
            .await
            .unwrap();
        assert!(ui.page == Page::Home);
        assert!(ui.values[1].is_empty());
        finish(&mut ui, &mut worker).await;
        assert!(ui.signed_in);
        assert!(!ui.failed);
        let key = service.dat_key().unwrap().unwrap();
        let key_lifetime = std::sync::Arc::downgrade(&key);
        drop(key);
        assert!(text(&ui, 80, 24).contains("MATCH DISTRIBUTION"));
        let cfg = service.settings().unwrap();
        assert!(cfg.url.is_empty());
        assert!(cfg.reset_token_url.is_empty());
        assert!(cfg.oauth);
        assert!(!std::fs::read_to_string(service.root.join("settings.json"))
            .unwrap()
            .contains("fresh-login-secret"));
        handle_key(
            &app,
            &service,
            &mut ui,
            &mut worker,
            KeyCode::Char('n').into(),
        )
        .await
        .unwrap();
        assert!(ui.page == Page::NewJob);
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Esc.into())
            .await
            .unwrap();
        handle_key(
            &app,
            &service,
            &mut ui,
            &mut worker,
            KeyCode::Char('l').into(),
        )
        .await
        .unwrap();
        assert!(!ui.signed_in);
        assert!(ui.page == Page::Home);
        assert!(ui.values[1].is_empty());
        assert_eq!(ui.values[0], "fixture");
        assert_eq!(ui.values[2], fixture.token_url());
        assert!(ui.values[3].is_empty());
        assert!(key_lifetime.upgrade().is_none());
        assert!(service.dat_key().unwrap().is_none());
        assert!(!Ui::new(cfg).signed_in);
    }
    #[tokio::test]
    async fn rejected_and_cancelled_login_stays_locked_and_can_retry() {
        for cancelled in [false, true] {
            let tmp = tempfile::tempdir().unwrap();
            let app = login_app(tmp.path());
            let service = app.matching().unwrap();
            let fixture = crate::sim::Fixture::spawn().await;
            fixture
                .set_plan(crate::sim::Plan {
                    token_status: if cancelled { 200 } else { 401 },
                    token_delay: if cancelled {
                        std::time::Duration::from_millis(300)
                    } else {
                        std::time::Duration::ZERO
                    },
                    ..Default::default()
                })
                .await;
            let cfg = Settings {
                token_url: fixture.token_url(),
                client_id: "fixture".into(),
                allow_http_loopback: true,
                ..Settings::default()
            };
            let mut ui = Ui::new(cfg);
            let mut worker = None;
            ui.values[1] = "retry-secret".into();
            ui.values[3] = test_dat_key();
            ui.focus = Some(3);
            handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
                .await
                .unwrap();
            if cancelled {
                tokio::time::timeout(std::time::Duration::from_secs(2), async {
                    while fixture.hits_token.load(std::sync::atomic::Ordering::SeqCst) == 0 {
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .unwrap();
                handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Esc.into())
                    .await
                    .unwrap();
            }
            finish(&mut ui, &mut worker).await;
            assert!(!ui.signed_in);
            assert!(ui.failed);
            assert!(ui.values[1].is_empty());
            assert_eq!(ui.values[0], "fixture");
            assert!(!ui.message.contains("retry-secret"));
            fixture.set_plan(Default::default()).await;
            ui.values[1] = "retry-secret".into();
            ui.values[3] = test_dat_key();
            ui.focus = Some(3);
            handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
                .await
                .unwrap();
            finish(&mut ui, &mut worker).await;
            assert!(ui.signed_in);
        }
    }
    #[tokio::test]
    async fn profile_change_relocks_while_rule_and_appearance_changes_preserve_session() {
        let tmp = tempfile::tempdir().unwrap();
        let app = login_app(tmp.path());
        let service = app.matching().unwrap();
        let fixture = crate::sim::Fixture::spawn().await;
        let cfg = Settings {
            token_url: fixture.token_url(),
            client_id: "fixture".into(),
            allow_http_loopback: true,
            ..Settings::default()
        };
        service.save_settings(&cfg).unwrap();
        let mut ui = Ui::new(cfg);
        let mut worker = None;
        ui.values[1] = "test-secret".into();
        ui.values[3] = test_dat_key();
        ui.focus = Some(3);
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
            .await
            .unwrap();
        finish(&mut ui, &mut worker).await;
        ui.open(Page::Settings);
        ui.tab = 1;
        ui.settings_values();
        ui.values[0] = "identifier".into();
        save_ui_settings(&app, &service, &mut ui).unwrap();
        assert!(ui.signed_in);
        ui.tab = 3;
        ui.settings_values();
        save_ui_settings(&app, &service, &mut ui).unwrap();
        assert!(ui.signed_in);
        ui.tab = 0;
        ui.settings_values();
        ui.values[0] = "https://data.example.test/partners.dat".into();
        save_ui_settings(&app, &service, &mut ui).unwrap();
        assert!(!ui.signed_in);
        assert!(ui.page == Page::Home);
        assert!(ui.values[1].is_empty());
        assert!(ui.snapshot.is_none());
        assert!(action(&service, &mut ui, &mut worker, "/download")
            .await
            .is_err());
    }
    #[tokio::test]
    async fn home_login_never_accesses_keychain_even_when_store_is_unavailable() {
        struct Unavailable;
        impl crate::secrets::SecretStore for Unavailable {
            fn get(&self, _: &str) -> Result<Option<Secret>, Error> {
                panic!("Home must not read Keychain")
            }
            fn put(&self, _: &str, _: &Secret) -> Result<(), Error> {
                panic!("Home must not write Keychain")
            }
            fn delete(&self, _: &str) -> Result<(), Error> {
                panic!("Home must not delete Keychain credentials")
            }
        }
        let tmp = tempfile::tempdir().unwrap();
        let app = App::open(
            tmp.path().into(),
            std::sync::Arc::new(Unavailable),
            std::sync::Arc::new(crate::timeutil::SystemClock),
        )
        .unwrap();
        let service = app.matching().unwrap();
        let fixture = crate::sim::Fixture::spawn().await;
        let mut ui = Ui::new(Settings {
            allow_http_loopback: true,
            ..Default::default()
        });
        let mut worker = None;
        ui.values = vec![
            "fixture".into(),
            "do-not-persist-secret".into(),
            fixture.token_url(),
            test_dat_key(),
        ];
        ui.focus = Some(3);
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
            .await
            .unwrap();
        assert!(ui.busy);
        finish(&mut ui, &mut worker).await;
        assert!(ui.signed_in);
        assert!(!ui.failed);
        assert!(!ui.busy);
        assert!(ui.values.iter().all(String::is_empty));
        assert!(ui.message.contains("Signed in"));
        assert!(!std::fs::read_to_string(service.root.join("settings.json"))
            .unwrap()
            .contains("do-not-persist-secret"));
    }
    #[tokio::test]
    async fn authenticated_settings_cannot_save_a_different_login_identity_or_endpoint() {
        let tmp = tempfile::tempdir().unwrap();
        let app = login_app(tmp.path());
        let service = app.matching().unwrap();
        let cfg = Settings {
            client_id: "original".into(),
            token_url: "https://auth.example.test/token".into(),
            oauth: true,
            ..Default::default()
        };
        service.save_settings(&cfg).unwrap();
        let before = std::fs::read(service.root.join("settings.json")).unwrap();
        for change_id in [false, true] {
            let mut ui = Ui::new(cfg.clone());
            ui.signed_in = true;
            ui.open(Page::Settings);
            if change_id {
                ui.config.client_id = "replacement".into();
            } else {
                ui.config.token_url = "https://other.example.test/token".into();
            }
            assert!(save_ui_settings(&app, &service, &mut ui).is_err());
            assert_eq!(
                std::fs::read(service.root.join("settings.json")).unwrap(),
                before
            );
        }
    }
    #[tokio::test]
    async fn login_with_success_status_but_no_token_stays_on_locked_home() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/token", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0u8; 4096];
            let mut request = Vec::new();
            loop {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                request.extend_from_slice(&buffer[..n]);
                if let Some(at) = request.windows(4).position(|s| s == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..at]);
                    let length = headers
                        .lines()
                        .find_map(|s| {
                            s.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if request.len() >= at + 4 + length {
                        break;
                    }
                }
            }
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").await.unwrap();
        });
        let tmp = tempfile::tempdir().unwrap();
        let app = login_app(tmp.path());
        let service = app.matching().unwrap();
        let mut ui = Ui::new(Settings {
            allow_http_loopback: true,
            ..Default::default()
        });
        let mut worker = None;
        ui.values = vec!["fixture".into(), "fresh-input".into(), url, test_dat_key()];
        ui.focus = Some(3);
        handle_key(&app, &service, &mut ui, &mut worker, KeyCode::Enter.into())
            .await
            .unwrap();
        finish(&mut ui, &mut worker).await;
        server.await.unwrap();
        assert!(!ui.signed_in);
        assert!(ui.failed);
        assert!(ui.page == Page::Home);
        assert!(ui.values[1].is_empty());
        assert!(action(&service, &mut ui, &mut worker, "/new")
            .await
            .is_err());
    }
}
