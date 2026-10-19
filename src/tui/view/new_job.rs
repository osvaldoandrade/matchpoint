use super::{
    block, content_inset, form_columns, form_field, message, motion, panel, Model, FG, GREEN, RED,
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub(super) fn render(frame: &mut Frame, model: &Model) {
    let form = model.import_form.as_ref().unwrap();
    let border = block(" Parter Get Parter / NEW JOB ");
    let inner = content_inset(border.inner(frame.area()));
    frame.render_widget(border, frame.area());
    let rows = form_rows(model, inner);
    let hint = if form.focus == 2 {
        "Space validation"
    } else {
        "Tab next"
    };
    frame.render_widget(
        Paragraph::new(format!(" Field {}/3 · {hint}", form.focus + 1))
            .style(Style::default().fg(FG)),
        rows[0],
    );
    let fields = field_area(frame, rows[1]);
    render_fields(frame, model, fields);
    render_note(frame, model, rows[2]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" Create job "),
            Span::raw(" Back "),
        ]))
        .style(Style::default().fg(GREEN)),
        rows[3],
    );
    frame.render_widget(
        Paragraph::new(" Enter: create · Esc: back").style(Style::default().fg(FG)),
        rows[4],
    );
}

fn form_rows(model: &Model, area: Rect) -> std::rc::Rc<[Rect]> {
    Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(6),
        Constraint::Length(if model.error.is_some() {
            if area.width < 68 {
                5
            } else {
                3
            }
        } else {
            2
        }),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(area)
}

fn field_area(frame: &mut Frame, area: Rect) -> Rect {
    if area.width < 96 {
        return area;
    }
    let cols = form_columns(area);
    panel(
        frame,
        " CONTEXT ",
        "CSV input\n\nThreads limit concurrent calls.\nValidation is selected per job.",
        cols[1],
        0,
    );
    cols[0]
}

fn render_fields(frame: &mut Frame, model: &Model, area: Rect) {
    let form = model.import_form.as_ref().unwrap();
    let fields = [
        ("CSV path (no quotes)", form.path.as_str()),
        ("Threads (1–256)", form.threads.as_str()),
        (
            "CPF validation",
            ["11 digits", "Check digits", "None"][form.validation % 3],
        ),
    ];
    let visible = (area.height as usize / 3).max(1);
    let start = form.focus.saturating_sub(visible - 1);
    for (row, (i, field)) in fields
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .enumerate()
    {
        form_field::render(
            frame,
            Rect::new(area.x, area.y + row as u16 * 3, area.width, 3),
            *field,
            form_field::Focus {
                selected: form.focus == i,
                cursor: form.cursor(),
                editable: i < 2,
                accent: motion::accent(model),
            },
        );
    }
}

fn render_note(frame: &mut Frame, model: &Model, area: Rect) {
    let note = if let Some(error) = &model.error {
        format!("{}\n{}\n{}", error.title, error.detail, error.recovery)
    } else {
        " Creates a local job; R starts requests.".into()
    };
    message(
        frame,
        area,
        note,
        if model.error.is_some() { RED } else { FG },
    );
}
