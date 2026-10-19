use super::{appearance::AppearanceForm, configure::ConfigForm, forms, Model};
use crate::{
    engine::{App, ProcessingDefaults},
    error::Error,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Tabs share drafts. No tab change writes config or reads the Keychain.
pub(super) struct Settings {
    pub tab: usize,
    pub focus: Option<usize>,
    pub api: ConfigForm,
    pub appearance: AppearanceForm,
    pub threads: String,
    pub rps: String,
    pub batch_size: String,
    pub validation: usize,
    cursors: [Option<usize>; 3],
    pub dirty: [bool; 4],
    pub notice: Option<(usize, String)>,
    pub error_group: Option<usize>,
    pub load_warnings: [Option<String>; 4],
}
impl Settings {
    pub fn new(app: &App, model: &Model, tab: usize) -> Result<Self, Error> {
        let (defaults, processing_warning) = match app.processing_defaults() {
            Ok(value) => (value, None),
            Err(_) => (
                ProcessingDefaults::default(),
                Some("Saved processing defaults are invalid. Enter a replacement.".into()),
            ),
        };
        let (api, notice) = match app.api_config() {
            Ok(api) => (api, None),
            Err(_) => (
                crate::engine::AppConfig::default(),
                Some((
                    0,
                    "Saved API profile is invalid. Enter a replacement and secret.".into(),
                )),
            ),
        };
        Ok(Self {
            tab,
            focus: None,
            api: ConfigForm::new(api),
            appearance: AppearanceForm::new(model),
            threads: defaults.threads.to_string(),
            rps: defaults.rps.to_string(),
            batch_size: defaults.batch_size.to_string(),
            validation: match defaults.validation.as_str() {
                "length" => 0,
                "none" => 2,
                _ => 1,
            },
            cursors: [None; 3],
            dirty: [false; 4],
            notice,
            error_group: None,
            load_warnings: [None, None, processing_warning, None],
        })
    }
    fn field_count(&self) -> usize {
        [4, 1, 4, 5][self.tab]
    }
    fn api_index(&self, focus: usize) -> usize {
        if self.tab == 1 {
            3
        } else {
            [0, 1, 2, 4][focus]
        }
    }
    pub fn fields(&self) -> Vec<(String, String)> {
        match self.tab {
            0 => [0, 1, 2, 4]
                .into_iter()
                .map(|i| {
                    let (name, value) = self.api.fields()[i].clone();
                    (name.into(), value)
                })
                .collect(),
            1 => {
                let (name, value) = self.api.fields()[3].clone();
                vec![(name.into(), value)]
            }
            2 => vec![
                ("Threads (1–256)".into(), self.threads.clone()),
                ("Requests / second".into(), self.rps.clone()),
                ("Rows / batch".into(), self.batch_size.clone()),
                (
                    "CPF validation".into(),
                    super::copy::VALIDATION[self.validation].into(),
                ),
            ],
            _ => vec![
                (
                    "Font".into(),
                    if self.appearance.font_vga {
                        "IBM VGA 8x16"
                    } else {
                        "Menlo"
                    }
                    .into(),
                ),
                (
                    "Font size".into(),
                    format!("{} pt", self.appearance.font_size),
                ),
                (
                    "Motion".into(),
                    super::copy::SPEED[self.appearance.speed as usize].into(),
                ),
                (
                    "Splash artwork".into(),
                    self.appearance.scene.label().into(),
                ),
                ("Font preview".into(), "[ Open new Ghostty window ]".into()),
            ],
        }
    }
    pub fn cursor(&self) -> Option<usize> {
        let focus = self.focus?;
        if focus >= self.field_count() {
            return None;
        }
        match self.tab {
            0 | 1 => self.api.cursor(),
            2 if focus < 3 => self.cursors[focus],
            _ => None,
        }
    }
    pub fn is_text(&self, focus: usize) -> bool {
        self.tab == 1 || (self.tab == 0 && focus < 3) || (self.tab == 2 && focus < 3)
    }
    pub fn paste(&mut self, value: &str) {
        if let Some(focus) = self.focus.filter(|f| *f < self.field_count()) {
            match self.tab {
                0 | 1 => {
                    self.api.focus = self.api_index(focus);
                    self.api.paste(value);
                }
                2 if focus < 3 => {
                    let input = match focus {
                        0 => &mut self.threads,
                        1 => &mut self.rps,
                        _ => &mut self.batch_size,
                    };
                    forms::paste(input, &mut self.cursors[focus], value, 16);
                }
                _ => return,
            }
            self.dirty[self.tab] = true;
            self.notice = None;
        }
    }
    pub fn key(&mut self, key: KeyEvent) -> Option<&'static str> {
        let count = self.field_count();
        match key.code {
            KeyCode::Esc => return Some("close-settings"),
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => return Some("quit"),
            KeyCode::Char('s') if key.modifiers == KeyModifiers::CONTROL => {
                return Some("save-settings")
            }
            KeyCode::F(2) => {
                self.tab = (self.tab
                    + if key.modifiers.contains(KeyModifiers::SHIFT) {
                        3
                    } else {
                        1
                    })
                    % 4;
                self.focus = None;
            }
            KeyCode::Left | KeyCode::Right if self.focus.is_none() => {
                self.tab = (self.tab + if key.code == KeyCode::Left { 3 } else { 1 }) % 4;
            }
            KeyCode::Tab | KeyCode::Down => {
                self.focus = match self.focus {
                    None => Some(0),
                    Some(f) if f < count + 1 => Some(f + 1),
                    _ => None,
                }
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.focus = match self.focus {
                    None => Some(count + 1),
                    Some(0) => None,
                    Some(f) => Some(f - 1),
                }
            }
            KeyCode::Enter => match self.focus {
                Some(f) if f == count => return Some("save-settings"),
                Some(f) if f == count + 1 => return Some("close-settings"),
                Some(4) if self.tab == 3 => return Some("settings-ghostty"),
                _ => self.focus = Some(self.focus.map_or(0, |f| f + 1)),
            },
            _ => {
                if let Some(focus) = self.focus.filter(|f| *f < count) {
                    let before = self.fields();
                    match self.tab {
                        0 | 1 => {
                            self.api.focus = self.api_index(focus);
                            self.api.key(key);
                        }
                        2 if focus < 3 => {
                            let input = match focus {
                                0 => &mut self.threads,
                                1 => &mut self.rps,
                                _ => &mut self.batch_size,
                            };
                            forms::edit(input, &mut self.cursors[focus], key, 16);
                        }
                        2 if matches!(
                            key.code,
                            KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right
                        ) =>
                        {
                            self.validation = (self.validation
                                + if key.code == KeyCode::Left { 2 } else { 1 })
                                % 3
                        }
                        3 => {
                            self.appearance.focus = focus;
                            self.appearance.key(key);
                        }
                        _ => {}
                    }
                    if self.fields() != before {
                        self.dirty[self.tab] = true;
                        self.notice = None;
                    }
                }
            }
        }
        if let Some(focus) = self.focus.filter(|f| *f < count) {
            if self.tab < 2 {
                self.api.focus = self.api_index(focus);
            }
        }
        None
    }
    pub fn processing(&self) -> Result<ProcessingDefaults, Error> {
        let settings = ProcessingDefaults {
            threads: self
                .threads
                .parse()
                .map_err(|_| Error::Msg("Threads must be an integer from 1 to 256.".into()))?,
            rps: self
                .rps
                .parse()
                .map_err(|_| Error::Msg("RPS must be a positive number.".into()))?,
            batch_size: self.batch_size.parse().map_err(|_| {
                Error::Msg("Batch size must be an integer from 1 to 100000.".into())
            })?,
            validation: ["length", "checksum", "none"][self.validation].into(),
        };
        settings.validate()?;
        Ok(settings)
    }
}
