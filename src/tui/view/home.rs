use super::{
    block, content_inset, motion, notice_height, render_commands, render_help, render_input,
    render_notice, Model, FG,
};
use crate::tui::{copy, palette};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

pub(super) fn render(frame: &mut Frame, model: &Model) {
    let border = block(copy::HOME_TITLE);
    let inner = content_inset(border.inner(frame.area()));
    frame.render_widget(border, frame.area());
    let prompt = model.command_mode || !model.repl.is_empty();
    let rows = home_rows(model, inner, prompt);
    headline(frame, model, rows[0]);
    menu(frame, model, rows[1]);
    super::dashboard::render(frame, model, rows[2]);
    render_notice(frame, model, rows[3]);
    status(frame, model, rows[4]);
    footer(frame, model, rows[5], prompt);
    if model.help {
        render_help(frame, model, inner);
    }
}

fn home_rows(model: &Model, inner: Rect, prompt: bool) -> std::rc::Rc<[Rect]> {
    Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(notice_height(model, inner.width)),
        Constraint::Length(3),
        Constraint::Length(if prompt { 3 } else { 1 }),
    ])
    .split(inner)
}

fn headline(frame: &mut Frame, model: &Model, area: Rect) {
    frame.render_widget(
        Paragraph::new(vec![
            motion::title(&format!(" {}", copy::HOME_HEADLINE), model),
            Line::styled(
                format!(" {}", copy::HOME_SUBHEADLINE),
                Style::default().fg(palette::MUTED),
            ),
        ]),
        area,
    );
}

fn footer(frame: &mut Frame, model: &Model, area: Rect, prompt: bool) {
    if prompt {
        render_input(frame, model, area, true);
        render_commands(frame, model, area);
    } else {
        let labels = if area.width >= 60 {
            copy::HOME_FOOTER
        } else {
            copy::HOME_FOOTER_COMPACT
        };
        let mut text = String::new();
        for (index, group) in labels.split('\t').enumerate() {
            if index > 0 {
                let gap = 8 - Line::from(text.as_str()).width() % 8;
                text.push_str(&" ".repeat(gap));
            }
            text.push_str(group);
        }
        frame.render_widget(Paragraph::new(text).style(Style::default().fg(FG)), area);
    }
}

fn menu(frame: &mut Frame, model: &Model, area: Rect) {
    let border = block(" MENU ");
    let inner = border.inner(area);
    frame.render_widget(border, area);
    let items = menu_items(model);
    if inner.width < 48 {
        frame.render_widget(Paragraph::new(Line::from(items)), inner);
    } else {
        let columns = Layout::horizontal([Constraint::Ratio(1, 3); 3]).split(inner);
        for (item, column) in items.into_iter().zip(columns.iter().copied()) {
            let style = item.style;
            frame.render_widget(
                Paragraph::new(Line::from(item))
                    .alignment(Alignment::Center)
                    .style(style),
                column,
            );
        }
    }
}

fn menu_items(model: &Model) -> Vec<Span<'static>> {
    let items = copy::HOME_LINKS
        .iter()
        .enumerate()
        .map(|(index, name)| {
            Span::styled(
                format!(
                    "{}[{}] {name}",
                    if index == model.home_selection {
                        ">"
                    } else {
                        " "
                    },
                    copy::HOME_KEYS[index]
                ),
                if index == model.home_selection {
                    motion::selected(model)
                } else {
                    Style::default().fg(palette::MUTED)
                },
            )
        })
        .collect::<Vec<_>>();
    items
}

fn status(frame: &mut Frame, model: &Model, area: Rect) {
    let selected = model.home_selection.min(2);
    let title = format!(" {} ", copy::HOME_LINKS[selected]);
    frame.render_widget(
        Paragraph::new(copy::HOME_DESCRIPTIONS[selected])
            .block(block(&title).style(Style::default().bg(palette::SURFACE).fg(FG)))
            .style(Style::default().bg(palette::SURFACE).fg(FG))
            .wrap(Wrap { trim: true }),
        area,
    );
}
