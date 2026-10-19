use super::{clipped, input_window, safe, BG, FG};
use crate::tui::palette;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub(crate) struct Focus {
    pub selected: bool,
    pub cursor: Option<usize>,
    pub editable: bool,
    pub accent: Color,
}

pub(crate) fn render(frame: &mut Frame, area: Rect, field: (&str, &str), focus: Focus) {
    let (name, value) = field;
    let border = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focus.selected {
            focus.accent
        } else {
            palette::BORDER
        }))
        .title(Line::styled(
            format!(" {}{name} ", if focus.selected { "> " } else { "" }),
            Style::default().fg(if focus.selected {
                focus.accent
            } else {
                palette::MUTED
            }),
        ));
    let content = border.inner(area);
    let text = safe(value);
    let (text, col) = if focus.selected {
        input_window(&text, focus.cursor, content.width as usize)
    } else {
        (clipped(&text, content.width as usize), 0)
    };
    frame.render_widget(border.style(Style::default().bg(BG).fg(FG)), area);
    frame.render_widget(
        Paragraph::new(text).style(if focus.selected {
            Style::default().fg(FG).bg(palette::SELECTED)
        } else {
            Style::default().fg(FG).bg(BG)
        }),
        content,
    );
    if focus.selected && focus.editable {
        frame.set_cursor_position((content.x + col, content.y));
    }
}
