use super::Model;
use crate::{engine::App, error::Error};
use crossterm::event::{KeyCode, KeyEvent};

pub(crate) struct AppearanceForm {
    pub font_vga: bool,
    pub font_size: u16,
    pub speed: u8,
    pub focus: usize,
    pub scene: super::SceneKind,
}
impl AppearanceForm {
    pub fn new(model: &Model) -> Self {
        Self {
            font_vga: model.font_vga,
            font_size: model.font_size,
            speed: if model.motion {
                model.animation_speed
            } else {
                0
            },
            focus: 0,
            scene: model.scene,
        }
    }
    pub fn key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab | KeyCode::Down => self.focus = (self.focus + 1) % 5,
            KeyCode::BackTab | KeyCode::Up => self.focus = (self.focus + 4) % 5,
            KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') => match self.focus {
                0 => self.font_vga = !self.font_vga,
                1 => {
                    self.font_size = if key.code == KeyCode::Left {
                        self.font_size.saturating_sub(2).max(12)
                    } else {
                        self.font_size.saturating_add(2).min(28)
                    }
                }
                2 => self.speed = (self.speed + if key.code == KeyCode::Left { 3 } else { 1 }) % 4,
                3 => self.scene = self.scene.next(),
                _ => {}
            },
            _ => {}
        }
    }
}

pub(crate) fn profile(model: &Model) -> String {
    include_str!("../../packaging/ghostty/pgm-ansi.ghostty")
        .replace(
            "PxPlus IBM VGA 8x16",
            if model.font_vga {
                "PxPlus IBM VGA 8x16"
            } else {
                "Menlo"
            },
        )
        .replace(
            "font-size = 16",
            &format!("font-size = {}", model.font_size),
        )
}

pub(crate) fn open_ghostty(app: &App, model: &Model) -> Result<(), Error> {
    let exe = std::env::current_exe()?;
    if exe.file_stem().and_then(|name| name.to_str()) != Some("pgm-data") {
        return Err(Error::Msg(
            "Isolated preview: use packaging/ghostty/run-ansi.command to apply the font.".into(),
        ));
    }
    let font = include_bytes!("../../assets/fonts/PxPlus_IBM_VGA_8x16.ttf");
    let user_home =
        std::env::var_os("HOME").ok_or_else(|| Error::Msg("HOME is unavailable.".into()))?;
    let directory = std::path::PathBuf::from(user_home).join("Library/Fonts");
    std::fs::create_dir_all(&directory)?;
    let font_path = directory.join("PxPlus_IBM_VGA_8x16.ttf");
    if font_path.exists() {
        if std::fs::read(&font_path)?.as_slice() != font {
            return Err(Error::Msg(
                "Existing VGA font differs from the package. Check Settings > Appearance.".into(),
            ));
        }
    } else {
        std::fs::write(&font_path, font)?;
    }
    let path = app.paths.root.join("ghostty-ansi.config");
    std::fs::write(&path, profile(model))?;
    crate::error::restrict_file(&path)?;
    let status = std::process::Command::new("open")
        .args(["-na", "Ghostty", "--args", "--config-default-files=false"])
        .arg(format!("--config-file={}", path.display()))
        .arg(format!(
            "--env=PGM_DATA_HOME={}",
            app.paths.root.canonicalize()?.display()
        ))
        .arg("-e")
        .arg(exe)
        .arg("tui")
        .status()?;
    if !status.success() {
        return Err(Error::Msg(
            "Ghostty did not open. Check its installation and the saved profile.".into(),
        ));
    }
    Ok(())
}
