use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use zeroize::Zeroizing;

use crate::engine::{App, AppConfig};
use crate::error::{Error, Secret};

pub(super) struct ConfigForm {
    pub settings: AppConfig,
    secret: Zeroizing<String>,
    pub focus: usize,
    cursors: [Option<usize>; 4],
}

impl ConfigForm {
    pub fn paste(&mut self, value: &str) {
        if self.focus < 4 {
            let limit = match self.focus {
                0 => 256,
                3 => 8192,
                _ => 2048,
            };
            let mut cursor = self.cursors[self.focus];
            super::forms::paste(self.input(), &mut cursor, value, limit);
            self.cursors[self.focus] = cursor;
        }
    }
    pub fn new(settings: AppConfig) -> Self {
        Self {
            settings,
            secret: Zeroizing::new(String::new()),
            focus: 0,
            cursors: [None; 4],
        }
    }

    pub fn fields(&self) -> [(&'static str, String); 5] {
        [
            (super::copy::API_FIELDS[0], self.settings.client_id.clone()),
            (
                super::copy::API_FIELDS[1],
                self.settings.process_url.clone(),
            ),
            (super::copy::API_FIELDS[2], self.settings.token_url.clone()),
            (
                super::copy::API_FIELDS[3],
                "*".repeat(self.secret.chars().count()),
            ),
            (
                super::copy::API_FIELDS[4],
                if self.settings.remote_enabled {
                    "[x] enabled"
                } else {
                    "[ ] disabled"
                }
                .into(),
            ),
        ]
    }

    pub fn key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab | KeyCode::Down => self.focus = (self.focus + 1) % 5,
            KeyCode::BackTab | KeyCode::Up => self.focus = (self.focus + 4) % 5,
            KeyCode::Char(' ') if self.focus == 4 => {
                self.settings.remote_enabled = !self.settings.remote_enabled
            }
            KeyCode::Char('u')
                if self.focus < 4 && key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.input().clear();
                self.cursors[self.focus] = None;
            }
            _ if self.focus < 4 => {
                let limit = if self.focus == 0 {
                    256
                } else if self.focus == 3 {
                    8192
                } else {
                    2048
                };
                let mut cursor = self.cursors[self.focus];
                super::forms::edit(self.input(), &mut cursor, key, limit);
                self.cursors[self.focus] = cursor;
            }
            _ => {}
        }
    }

    pub fn cursor(&self) -> Option<usize> {
        if self.focus >= 4 {
            return None;
        }
        let at = self.cursors[self.focus];
        if self.focus == 3 {
            at.map(|at| self.secret[..at.min(self.secret.len())].chars().count())
        } else {
            at
        }
    }

    fn input(&mut self) -> &mut String {
        match self.focus {
            0 => &mut self.settings.client_id,
            1 => &mut self.settings.process_url,
            2 => &mut self.settings.token_url,
            _ => &mut self.secret,
        }
    }

    pub fn save(&self, app: &App) -> Result<(), Error> {
        if self.secret.is_empty() {
            return app.configure_api_metadata(self.settings.clone());
        }
        self.settings.validate()?;
        let secret = Secret::new(self.secret.to_string())?;
        app.configure_api(self.settings.clone(), secret, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_is_masked_and_keyboard_navigation_wraps() {
        let mut form = ConfigForm::new(AppConfig::default());
        form.focus = 3;
        for ch in "test-only-secret".chars() {
            form.key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        assert_eq!(form.fields()[3].1, "****************");
        form.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        form.key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert!(form.settings.remote_enabled);
        form.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(form.focus, 0);
        form.key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
        assert_eq!(form.focus, 4);
    }
}
