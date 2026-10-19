use super::{block, content_inset, motion, motion_phase, Model, FG};
use crate::tui::{art, copy, palette};
use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub(super) fn render(frame: &mut Frame, model: &Model) {
    let border = block(copy::HOME_TITLE);
    let inner = content_inset(border.inner(frame.area()));
    frame.render_widget(border, frame.area());
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(if inner.height >= 22 { 3 } else { 1 }),
        Constraint::Length(1),
    ])
    .split(inner);
    frame.render_widget(
        Paragraph::new(vec![
            motion::title(copy::HOME_HEADLINE, model),
            Line::styled(copy::HOME_SUBHEADLINE, Style::default().fg(palette::MUTED)),
        ])
        .alignment(Alignment::Center),
        rows[0],
    );
    frame.render_widget(
        art::SplashScene {
            phase: motion_phase(model),
            motion: model.motion,
            kind: model.scene,
        },
        rows[1],
    );
    let button = Paragraph::new(Line::from(Span::styled(
        " Enter to open workspace ",
        motion::selected(model),
    )))
    .alignment(Alignment::Center);
    if rows[2].height >= 3 {
        frame.render_widget(button.block(block(" CONTINUE ")), rows[2]);
    } else {
        frame.render_widget(button, rows[2]);
    }
    frame.render_widget(
        Paragraph::new("Esc quit")
            .alignment(Alignment::Center)
            .style(Style::default().fg(FG)),
        rows[3],
    );
}

pub(in crate::tui) fn key(model: &mut Model, key: crossterm::event::KeyEvent) -> Option<String> {
    use crossterm::event::{KeyCode, KeyModifiers};
    match key.code {
        KeyCode::Enter => {
            model.splash = false;
            model.home = true;
        }
        KeyCode::Char(' ') => model.motion = !model.motion,
        KeyCode::Esc | KeyCode::Char('q' | 'Q') => return Some("quit".into()),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Some("quit".into())
        }
        _ => {}
    }
    None
}
