use serde::{Deserialize, Serialize};

use super::Model;
use crate::engine::App;
use crate::error::Error;
use crate::repl::sensitive;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Session {
    selected_job: Option<String>,
    filter: String,
    #[serde(default = "motion_default")]
    motion: bool,
    #[serde(default = "font_size_default")]
    font_size: u16,
    #[serde(default = "motion_default")]
    font_vga: bool,
    #[serde(default = "speed_default")]
    speed: u8,
    #[serde(default)]
    scene: super::SceneKind,
}

fn motion_default() -> bool {
    true
}
fn font_size_default() -> u16 {
    16
}
fn speed_default() -> u8 {
    2
}

impl Default for Session {
    fn default() -> Self {
        Self {
            selected_job: None,
            filter: "all".into(),
            motion: true,
            font_size: 16,
            font_vga: true,
            speed: 2,
            scene: super::SceneKind::default(),
        }
    }
}

pub(crate) fn load_session(app: &App, model: &mut Model) -> Result<(), Error> {
    let path = app.paths.session_file();
    if path.exists() {
        if let Ok(session) = serde_json::from_slice::<Session>(&std::fs::read(&path)?) {
            model.filter = session.filter;
            model.selected_job = session.selected_job;
            model.motion = session.motion;
            model.font_size = session.font_size.clamp(12, 28);
            model.font_vga = session.font_vga;
            model.animation_speed = session.speed.clamp(1, 3);
            model.scene = session.scene;
        }
    }
    let history = app.paths.history_file();
    if history.exists() {
        let text = std::fs::read_to_string(history)?;
        model.history = text
            .lines()
            .filter(|line| !sensitive(line))
            .map(|s| s.to_string())
            .collect();
    }
    Ok(())
}

pub(crate) fn save_session(app: &App, model: &Model) -> Result<(), Error> {
    let session = Session {
        selected_job: model.selected_job.clone(),
        filter: model.filter.clone(),
        motion: model.motion,
        font_size: model.font_size,
        font_vga: model.font_vga,
        speed: model.animation_speed,
        scene: model.scene,
    };
    let path = app.paths.session_file();
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&session).map_err(|e| Error::Msg(e.to_string()))?,
    )?;
    crate::error::restrict_file(&path)?;
    let history: Vec<&str> = model
        .history
        .iter()
        .filter(|line| !sensitive(line))
        .map(String::as_str)
        .collect();
    let path = app.paths.history_file();
    std::fs::write(&path, history.join("\n"))?;
    crate::error::restrict_file(&path)?;
    Ok(())
}
