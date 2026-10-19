use super::{feedback::safe, Model};
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap};
use ratatui::Frame;

mod dashboard;
pub(crate) mod form_field;
mod home;
pub(crate) mod motion;
mod new_job;
pub(super) mod splash;

// Graphite surfaces and restrained jade accents preserve semantic status colors.
const BG: Color = super::palette::BG;
const FG: Color = super::palette::FG;
const MUTED: Color = super::palette::MUTED;
const GREEN: Color = super::palette::BRIGHT;
const YELLOW: Color = super::palette::WARNING;
const RED: Color = super::palette::DANGER;

pub fn state_label(state: &str) -> &str {
    match state {
        "PREPARE_FAILED" => "Import failed",
        "PREPARED" => "Ready",
        "RUNNING" => "Running",
        "PAUSED" => "Paused",
        "INTERRUPTED" => "Interrupted",
        "COMPLETED" => "Completed",
        "COMPLETED_WITH_ERRORS" => "Completed with errors",
        "BLOCKED_UNKNOWN" => "Uncertain results",
        "PREPARING" => "Preparing",
        "PENDING" => "Pending",
        _ => state,
    }
}

pub(crate) fn block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(super::palette::BORDER))
        .style(Style::default().bg(BG).fg(FG))
        .title(Line::styled(title, Style::default().fg(FG).bold()))
}
fn panel(frame: &mut Frame, title: &str, text: impl Into<String>, area: Rect, scroll: u16) {
    let lines = wrapped(&safe(&text.into()), area.width.saturating_sub(2) as usize);
    let count = lines.len();
    let paragraph = Paragraph::new(lines);
    let height = area.height.saturating_sub(2) as usize;
    let offset = (scroll as usize).min(count.saturating_sub(height));
    let border = if count > height {
        block(title).title_bottom(format!(
            " {}–{}/{} · PgUp/Dn ",
            offset + 1,
            (offset + height).min(count),
            count
        ))
    } else {
        block(title)
    };
    frame.render_widget(paragraph.scroll((offset as u16, 0)).block(border), area);
}

fn wrapped(text: &str, width: usize) -> Vec<Line<'static>> {
    let width = width.max(1);
    let mut out = Vec::new();
    for raw in text.split('\n') {
        let mut current = String::new();
        for ch in raw.chars() {
            if Line::from(format!("{current}{ch}")).width() > width && !current.is_empty() {
                if let Some(at) = current.rfind(' ').filter(|at| *at > 0) {
                    out.push(Line::from(current[..at].to_string()));
                    current = current[at + 1..].to_string();
                } else {
                    out.push(Line::from(std::mem::take(&mut current)));
                }
            }
            current.push(ch);
        }
        out.push(Line::from(current));
    }
    out
}

fn content_inset(area: Rect) -> Rect {
    area.inner(Margin::new(1, 0))
}

fn form_columns(area: Rect) -> std::rc::Rc<[Rect]> {
    Layout::horizontal([Constraint::Percentage(65), Constraint::Percentage(35)])
        .spacing(1)
        .split(area)
}

fn message(frame: &mut Frame, area: Rect, text: impl Into<String>, color: Color) {
    frame.render_widget(
        Paragraph::new(text.into())
            .block(Block::default().padding(Padding::horizontal(1)))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(color)),
        area,
    );
}

fn notice_height(model: &Model, width: u16) -> u16 {
    if model.error.is_some() {
        if width < 70 {
            5
        } else {
            3
        }
    } else if model.working || model.active_job.is_some() {
        2
    } else if model.message.is_empty() && !model.busy {
        0
    } else {
        1
    }
}

fn workspace_rows(area: Rect, model: &Model) -> std::rc::Rc<[Rect]> {
    Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(notice_height(model, area.width)),
        Constraint::Min(5),
        Constraint::Length(if model.command_mode || !model.repl.is_empty() {
            3
        } else {
            1
        }),
        Constraint::Length(if area.width >= 80 { 1 } else { 2 }),
    ])
    .split(area)
}

fn detail_area(model: &Model) -> Rect {
    let area = Rect::new(0, 0, model.viewport.0, model.viewport.1);
    let workspace = workspace_rows(content_inset(block("").inner(area)), model)[2];
    if model.viewport.0 >= 80 {
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(workspace)[1]
    } else {
        workspace
    }
}

fn detail_rows(area: Rect, summary_height: usize) -> std::rc::Rc<[Rect]> {
    let summary_height = summary_height as u16;
    // Keep panel borders, the batch heading and at least one row of useful content.
    let tabs = u16::from(area.height > summary_height.saturating_add(4));
    let progress = area.height.saturating_sub(tabs + summary_height + 4).min(2);
    Layout::vertical([
        Constraint::Length(tabs),
        Constraint::Length(summary_height),
        Constraint::Length(progress),
        Constraint::Min(4),
    ])
    .split(area)
}

pub(super) fn batch_page_size(model: &Model) -> u32 {
    let area = detail_area(model);
    detail_rows(area, detail_summary(model, area.width).len())[3]
        .height
        .saturating_sub(3)
        .max(1) as u32
}

pub(super) fn detail_scroll_step(model: &Model) -> u16 {
    if model.home || model.help || model.panel == 0 {
        return 5;
    }
    let area = detail_area(model);
    detail_rows(area, detail_summary(model, area.width).len())[3]
        .height
        .saturating_sub(2)
        .clamp(1, 5)
}

pub fn render(frame: &mut Frame, model: &Model) {
    let area = frame.area();
    frame.render_widget(Block::default().style(Style::default().bg(BG).fg(FG)), area);
    if area.width < 40 || area.height < 18 {
        render_compact(frame, model);
        return;
    }
    if model.splash {
        splash::render(frame, model);
        return;
    }
    if model.settings.is_some() {
        render_settings(frame, model);
        return;
    }
    if model.import_form.is_some() {
        new_job::render(frame, model);
        return;
    }
    if model.form.is_some()
        || model.thread_form.is_some()
        || model.export_form.is_some()
        || model.appearance_form.is_some()
    {
        render_form(frame, model);
        return;
    }
    if model.home {
        home::render(frame, model);
        return;
    }
    let border = block(" Parter Get Parter / JOBS ");
    let inner = content_inset(border.inner(area));
    frame.render_widget(border, area);
    let rows = workspace_rows(inner, model);
    render_header(frame, rows[0], false);
    render_notice(frame, model, rows[1]);
    render_panels(frame, model, rows[2]);
    if model.command_mode || !model.repl.is_empty() {
        render_input(frame, model, rows[3], true);
    } else {
        frame.render_widget(
            Paragraph::new(if area.width >= 80 {
                " R run/resume  P pause  T threads  E export  K checksum"
            } else {
                " R run  P pause  T threads  E export"
            })
            .style(Style::default().fg(GREEN)),
            rows[3],
        );
    }
    frame.render_widget(
        Paragraph::new(if area.width >= 80 {
            " H home  N new  S settings  Q quit   ↑↓ jobs  Tab details  ? help"
        } else if model.panel == 0 {
            " N new  S settings  H home  Q quit\n ↑↓ select  Enter details  ? help"
        } else {
            " K checksum S settings H home Q quit\n Tab details  Esc jobs  PgUp/Dn scroll"
        })
        .style(Style::default().fg(FG)),
        rows[4],
    );
    if model.help {
        render_help(frame, model, rows[2]);
    } else {
        render_commands(frame, model, rows[3]);
    }
}

fn render_header(frame: &mut Frame, area: Rect, _large: bool) {
    frame.render_widget(
        Paragraph::new(" ↑↓ select job · Tab switch details")
            .style(Style::default().fg(GREEN).bold()),
        area,
    );
}

fn render_notice(frame: &mut Frame, model: &Model, area: Rect) {
    let (text, color) = if let Some(error) = &model.error {
        (
            format!(
                "[ERROR] {}\n{}\n{}",
                error.title,
                error.detail.lines().next().unwrap_or(""),
                error.recovery
            ),
            RED,
        )
    } else if model.working {
        (
            format!(
                "{}\n{}",
                model.work_title,
                indeterminate(area.width.saturating_sub(14) as usize, model)
            ),
            GREEN,
        )
    } else if let Some(id) = &model.active_job {
        (
            format!(
                "ACTIVE {} · P pauses this job\n{}",
                &id[..8],
                safe(&model.message)
            ),
            GREEN,
        )
    } else {
        (
            safe(&model.message),
            if model.message.contains("MISMATCH") {
                RED
            } else {
                FG
            },
        )
    };
    message(frame, area, text, color);
}

fn render_panels(frame: &mut Frame, model: &Model, area: Rect) {
    if frame.area().width >= 80 {
        let cols = Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);
        render_jobs(frame, model, cols[0]);
        render_detail(frame, model, cols[1]);
    } else if model.panel == 0 {
        render_jobs(frame, model, area);
    } else {
        render_detail(frame, model, area);
    }
}

fn detail_summary(model: &Model, width: u16) -> Vec<Line<'static>> {
    let Some(job) = model
        .jobs
        .iter()
        .find(|job| Some(&job.id) == model.selected_job.as_ref())
    else {
        return vec![Line::default()];
    };
    let done = job.counts.succeeded + job.counts.failed_final + job.counts.unknown;
    let identity = format!(" {} · {}", &job.id[..8], state_label(&job.state));
    let counter = format!("{done}/{}", job.counts.total());
    let single = format!("{identity} · {counter}");
    if Line::from(single.as_str()).width() <= width as usize {
        return vec![Line::from(single)];
    }
    let mut lines = wrapped(&identity, width as usize);
    if counter.len() < width as usize {
        lines.push(Line::from(format!(" {counter}")));
    } else {
        lines.extend(wrapped(
            &format!(" Done {done}\n Total {}", job.counts.total()),
            width as usize,
        ));
    }
    lines
}

fn render_detail(frame: &mut Frame, model: &Model, area: Rect) {
    let summary = detail_summary(model, area.width);
    let rows = detail_rows(area, summary.len());
    let names = super::copy::JOB_TABS;
    let tabs = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            Span::styled(
                format!(" {name} "),
                if model.panel.max(1) - 1 == i {
                    motion::selected(model)
                } else {
                    Style::default().fg(FG)
                },
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(Line::from(tabs)), rows[0]);
    frame.render_widget(
        Paragraph::new(summary).style(Style::default().fg(FG)),
        rows[1],
    );
    if let Some(job) = model
        .jobs
        .iter()
        .find(|j| Some(&j.id) == model.selected_job.as_ref())
    {
        let done = job.counts.succeeded + job.counts.failed_final + job.counts.unknown;
        frame.render_widget(
            Paragraph::new(bar(
                job.counts.total(),
                done,
                area.width.saturating_sub(8) as usize,
                job.state == "RUNNING",
                model,
            )),
            rows[2],
        );
    }
    match model.panel.max(1) {
        1 => render_dashboard(frame, model, rows[3]),
        2 => render_batches(frame, model, rows[3]),
        3 => render_events(frame, model, rows[3]),
        _ => render_checksum(frame, model, rows[3]),
    }
}

fn render_jobs(frame: &mut Frame, model: &Model, area: Rect) {
    if model.jobs.is_empty() {
        panel(
            frame,
            " JOBS ",
            "No jobs yet.\n\nN: create a job from CSV\nH: home",
            area,
            model.scroll,
        );
        return;
    }
    let inner = block("").inner(area);
    let visible = (inner.height as usize / 3).max(1);
    let selected = model
        .jobs
        .iter()
        .position(|job| Some(&job.id) == model.selected_job.as_ref())
        .unwrap_or(0);
    let start = selected.saturating_sub(visible - 1);
    let title = format!(
        " JOBS {}–{}/{} ",
        start + 1,
        (start + visible).min(model.jobs.len()),
        model.jobs.len()
    );
    let mut lines = Vec::new();
    for job in model.jobs.iter().skip(start).take(visible) {
        let focused = Some(&job.id) == model.selected_job.as_ref();
        let filename = model
            .job_configs
            .iter()
            .find(|cfg| cfg.job_id == job.id)
            .map(|cfg| safe(&cfg.input_name))
            .unwrap_or_default();
        lines.push(Line::styled(
            clipped(
                &format!(
                    "{} {} {}",
                    if focused { ">" } else { " " },
                    &job.id[..8],
                    filename
                ),
                inner.width as usize,
            ),
            if focused {
                motion::selected(model)
            } else {
                Style::default().fg(FG)
            },
        ));
        let status = format!(
            "  {}{}",
            state_label(&job.state),
            if job.desired == "pause" && job.state == "RUNNING" {
                " / PAUSING"
            } else {
                ""
            }
        );
        lines.push(Line::styled(status, Style::default().fg(FG)));
        let done = job.counts.succeeded + job.counts.failed_final + job.counts.unknown;
        lines.push(bar(
            job.counts.total(),
            done,
            inner.width.saturating_sub(7) as usize,
            job.state == "RUNNING",
            model,
        ));
    }
    frame.render_widget(Paragraph::new(lines).block(block(&title)), area);
}

fn render_dashboard(frame: &mut Frame, model: &Model, area: Rect) {
    let text = if let Some(job) = model
        .jobs
        .iter()
        .find(|job| Some(&job.id) == model.selected_job.as_ref())
    {
        let c = &job.counts;
        let cfg = model.selected_config.as_ref();
        format!("{}\n{} · {}\n\nSucceeded {} · failed {}\nUncertain {} · pending {}\nReserved {} · retry {}\n{} threads · RPS {}\n\n{}",cfg.map(|c|safe(&c.input_name)).unwrap_or_default(),&job.id[..8],state_label(&job.state),c.succeeded,c.failed_final,c.unknown,c.pending,c.in_flight,c.retry_wait,cfg.map(|c|c.concurrency).unwrap_or(0),cfg.map(|c|c.rps).unwrap_or(0.),next_action(model))
    } else {
        model.dashboard.clone()
    };
    panel(frame, " OVERVIEW ", text, area, model.scroll);
}

fn next_action(model: &Model) -> &'static str {
    match model
        .jobs
        .iter()
        .find(|job| Some(&job.id) == model.selected_job.as_ref())
        .map(|job| job.state.as_str())
    {
        Some("RUNNING") => "P: pause active job",
        Some("COMPLETED") => "E: export results",
        Some("COMPLETED_WITH_ERRORS") => "Events: inspect failures · ? recovery",
        Some("BLOCKED_UNKNOWN") => "Uncertain results. E: partial export.",
        Some("PAUSED" | "INTERRUPTED") => "R: resume",
        Some("PREPARE_FAILED") => "job prepare <id>: retry import",
        _ => "R: run",
    }
}

fn render_batches(frame: &mut Frame, model: &Model, area: Rect) {
    let inner = block("").inner(area);
    let visible = inner.height.saturating_sub(1).max(1) as usize;
    let title = format!(
        " BATCHES {}–{} · PgUp/Dn ",
        model.batch_offset + 1,
        model.batch_offset + model.batch_items.len().min(visible) as u32
    );
    if model.batch_items.is_empty() {
        panel(frame, " BATCHES ", "No batches match this filter.", area, 0);
        return;
    }
    let compact = inner.width < 58;
    let mut lines = vec![Line::styled(
        if compact {
            " ID   STATE             DONE"
        } else {
            " ID    STATE               DONE       PROGRESS"
        },
        Style::default().fg(GREEN),
    )];
    for batch in model.batch_items.iter().take(visible) {
        let total = batch.row_to - batch.row_from + 1;
        let done = batch.succeeded + batch.failed + batch.unknown;
        let label = state_label(&batch.state);
        let label = label
            .chars()
            .take(if compact { 16 } else { 20 })
            .collect::<String>();
        let mut spans = vec![Span::raw(format!(
            " {:<4} {:<width$} {done}/{total}",
            batch.batch_id,
            label,
            width = if compact { 16 } else { 20 }
        ))];
        if !compact {
            spans.push(Span::raw("  "));
            spans.extend(
                bar(
                    total,
                    done,
                    inner.width.saturating_sub(48).max(3) as usize,
                    batch.in_flight > 0,
                    model,
                )
                .spans,
            );
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines).block(block(&title)), area);
}

fn render_events(frame: &mut Frame, model: &Model, area: Rect) {
    let mut text = model
        .events
        .iter()
        .rev()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    if !model.job_events.is_empty() {
        text.push_str("\n\nJOB EVENTS\n");
        text.push_str(&model.job_events.join("\n"));
    }
    panel(frame, " EVENTS · PgUp/Dn ", text, area, model.scroll);
}

fn render_checksum(frame: &mut Frame, model: &Model, area: Rect) {
    let text = if let Some(report) = &model.checksum {
        let mut text = format!(
            "SHA-256 · checked at {}\n{}\n",
            report.checked_at,
            if report.matches() {
                "MATCH: hashes agree"
            } else {
                "MISMATCH: file changed/missing"
            }
        );
        for artifact in &report.artifacts {
            text.push_str(&format!(
                "\n{} [{}]\nExpected:\n{}\nActual:\n{}\n",
                artifact.name,
                if artifact.matches { "OK" } else { "MISMATCH" },
                artifact.expected,
                artifact.actual
            ));
        }
        text.push_str("\nChecks stored input.csv and published exports/gen-*/merged.*.\nMismatch: restore the original or N: create a new job.\nK: check again. Results describe the check time.");
        text
    } else if let Some(config) = &model.selected_config {
        format!("Stored input.csv SHA-256\nNOT CHECKED in this session\n\nExpected at import:\n{}\n\nK: check input and exports.\n\nCPF validation: {} (separate from SHA-256).", config.input_sha256, config.validation)
    } else {
        "Select a job. K checks stored files.".into()
    };
    panel(frame, " INTEGRITY / SHA-256 ", text, area, model.scroll);
}

fn motion_phase(model: &Model) -> u64 {
    model.animation
        / match model.animation_speed {
            1 => 4,
            3 => 1,
            _ => 2,
        }
}

fn bar(total: i64, done: i64, width: usize, active: bool, model: &Model) -> Line<'static> {
    let ratio = if total > 0 {
        (done as f64 / total as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let width = width.max(3);
    let filled = (ratio * width as f64).floor() as usize;
    let mut cells = "█".repeat(filled);
    if filled < width {
        cells.push(if active && model.motion && motion_phase(model) % 2 == 1 {
            '▓'
        } else {
            '░'
        });
        cells.push_str(&"░".repeat(width - filled - 1));
    }
    Line::from(vec![
        Span::styled(format!("[{cells}]"), Style::default().fg(GREEN)),
        Span::styled(format!(" {:3.0}%", ratio * 100.0), Style::default().fg(FG)),
    ])
}

fn indeterminate(width: usize, model: &Model) -> String {
    let width = width.clamp(3, 60);
    let position = if model.motion {
        motion_phase(model) as usize % width
    } else {
        0
    };
    format!(
        "[{}▓{}] working",
        "░".repeat(position),
        "░".repeat(width - position - 1)
    )
}

fn render_compact(frame: &mut Frame, model: &Model) {
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(frame.area());
    frame.render_widget(
        Paragraph::new("Parter Get Parter · ? help").style(Style::default().fg(GREEN)),
        rows[0],
    );
    let text = format!("Terminal: use at least 40×18.\n{}", safe(&model.message));
    frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), rows[1]);
    render_input(frame, model, rows[2], false);
    render_commands(frame, model, rows[2]);
}

fn render_input(frame: &mut Frame, model: &Model, area: Rect, bordered: bool) {
    let inner = if bordered {
        block("").inner(area)
    } else {
        area
    };
    let text = safe_command(&model.repl);
    let at = if crate::repl::sensitive(&model.repl) {
        text.len()
    } else {
        model
            .input_cursor
            .unwrap_or(model.repl.len())
            .min(model.repl.len())
    };
    let prefix: String = text
        .chars()
        .take_while({
            let mut bytes = 0;
            move |ch| {
                bytes += ch.len_utf8();
                bytes <= at
            }
        })
        .collect();
    let available = inner.width.saturating_sub(3) as usize;
    let skip = prefix.chars().count().saturating_sub(available);
    let visible: String = text.chars().skip(skip).take(available + 1).collect();
    let paragraph = Paragraph::new(format!("> {visible}"));
    frame.render_widget(
        if bordered {
            paragraph.block(block(" COMMAND · Enter submit "))
        } else {
            paragraph
        },
        area,
    );
    if inner.width > 2 && inner.height > 0 {
        let col = prefix.chars().count().saturating_sub(skip).min(available) as u16;
        frame.set_cursor_position((inner.x + 2 + col, inner.y));
    }
}
fn safe_command(line: &str) -> String {
    if crate::repl::sensitive(line) {
        "[secret hidden — use Settings]".into()
    } else {
        safe(line)
    }
}

fn render_commands(frame: &mut Frame, model: &Model, input: Rect) {
    let Some(items) = super::commands::suggestions(model) else {
        return;
    };
    let height =
        ((items.len().max(1) + 2).min(9) as u16).min(input.y.saturating_sub(frame.area().y));
    if height < 3 || input.width < 4 {
        return;
    }
    let area = Rect::new(input.x, input.y - height, input.width.min(90), height);
    let selected = model.completion_at.min(items.len().saturating_sub(1));
    let visible = height.saturating_sub(2) as usize;
    let start = selected.saturating_sub(visible - 1);
    let mut lines: Vec<Line> = items
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .map(|(index, cmd)| {
            let marker = if index == selected { ">" } else { " " };
            let text = if area.width >= 50 {
                format!("{marker} {:16} {}", cmd.name, cmd.description)
            } else {
                format!("{marker} {}", cmd.name)
            };
            Line::styled(
                text,
                if index == selected {
                    motion::selected(model)
                } else {
                    Style::default().fg(FG)
                },
            )
        })
        .collect();
    if items.is_empty() {
        lines.push(Line::styled(
            "No commands · ? help",
            Style::default().fg(YELLOW),
        ));
    }
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines).block(block(if area.width < 70 {
            " / · ↑↓ · Tab · Esc "
        } else {
            " / · ↑↓ select · Tab complete · Esc close "
        })),
        area,
    );
}

fn clipped(value: &str, width: usize) -> String {
    if Line::from(value).width() <= width {
        return value.into();
    }
    let mark = ".".repeat(width.min(3));
    let limit = width.saturating_sub(mark.len());
    let mut out = String::new();
    let mut used = 0;
    for ch in value.chars() {
        let cw = Line::from(ch.to_string()).width();
        if used + cw > limit {
            break;
        }
        out.push(ch);
        used += cw;
    }
    out + &mark
}

fn input_window(value: &str, at: Option<usize>, width: usize) -> (String, u16) {
    let width = width.max(1);
    let mut at = at.unwrap_or(value.len()).min(value.len());
    while !value.is_char_boundary(at) {
        at = at.saturating_sub(1);
    }
    let prefix = Line::from(value[..at].to_string()).width();
    let target = prefix.saturating_sub(width.saturating_sub(1));
    let mut start = 0;
    let mut skipped = 0;
    for (i, ch) in value.char_indices() {
        if skipped >= target {
            start = i;
            break;
        }
        skipped += Line::from(ch.to_string()).width();
        start = i + ch.len_utf8();
    }
    let mut text = String::new();
    let mut used = 0;
    for ch in value[start..].chars() {
        let cw = Line::from(ch.to_string()).width();
        if used + cw > width {
            break;
        }
        text.push(ch);
        used += cw;
    }
    (text, (prefix.saturating_sub(skipped).min(width - 1)) as u16)
}

fn render_settings(frame: &mut Frame, model: &Model) {
    let settings = model.settings.as_ref().unwrap();
    let error = model.error.as_ref().filter(|_| {
        settings
            .error_group
            .is_none_or(|group| group == if settings.tab < 2 { 0 } else { settings.tab })
    });
    let border = block(" Parter Get Parter / SETTINGS ");
    let inner = content_inset(border.inner(frame.area()));
    frame.render_widget(border, frame.area());
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(6),
        Constraint::Length(if error.is_some() {
            if inner.width < 68 {
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
    .split(inner);
    let tabs = std::iter::once(Span::raw(" "))
        .chain(
            super::copy::SETTINGS_TABS
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    Span::styled(
                        format!("{name}{} ", if settings.dirty[i] { "*" } else { "" }),
                        if i == settings.tab {
                            motion::selected(model)
                        } else {
                            Style::default().fg(FG)
                        },
                    )
                }),
        )
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(Line::from(tabs)), rows[0]);
    let count = settings.fields().len();
    let visible = (rows[2].height as usize / 3).max(1);
    let active = settings.focus.unwrap_or(0).min(count - 1);
    let start = active.saturating_sub(visible - 1);
    let hint = match settings.focus {
        None => format!(
            " ←/→ tabs · Tab edit · {}/{} fields",
            (start + visible).min(count),
            count
        ),
        Some(f) if f < count => format!(" Field {}/{} · Tab next · F2 tabs", f + 1, count),
        _ => " Enter activates · Shift+Tab fields".into(),
    };
    frame.render_widget(Paragraph::new(hint).style(Style::default().fg(FG)), rows[1]);
    let field_area = if inner.width >= 96 {
        let cols = form_columns(rows[2]);
        if settings.tab == 3 {
            let preview = block(" ARTWORK PREVIEW ");
            let content = preview.inner(cols[1]);
            frame.render_widget(preview, cols[1]);
            frame.render_widget(
                super::art::SplashScene {
                    phase: 0,
                    motion: false,
                    kind: settings.appearance.scene,
                },
                content,
            );
        } else {
            panel(frame," CONTEXT ",match settings.tab {
                0=>"HTTPS endpoints\nRemote requests require explicit enablement.\n\nChanged identity or URLs need a new secret.",
                1=>"Stored in macOS Keychain.\nNever shown or added to history.\n\nA blank field keeps the secret for the same profile.",
                _=>"Defaults for NEW jobs.\n\nThreads limit concurrent calls.\nRPS limits sends per second.\nReserved includes the worker queue.",
            },cols[1],0);
        }
        cols[0]
    } else {
        rows[2]
    };
    let fields = settings.fields();
    let count = fields.len();
    let visible = (field_area.height as usize / 3).max(1);
    let focus = settings.focus.unwrap_or(0).min(count - 1);
    let start = focus.saturating_sub(visible - 1);
    for (row, (i, (name, value))) in fields
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .enumerate()
    {
        let selected = settings.focus == Some(i);
        let area = Rect::new(
            field_area.x,
            field_area.y + row as u16 * 3,
            field_area.width,
            3,
        );
        form_field::render(
            frame,
            area,
            (name, value),
            form_field::Focus {
                selected,
                cursor: settings.cursor(),
                editable: settings.is_text(i),
                accent: motion::accent(model),
            },
        );
    }
    let note = if let Some(error) = error {
        format!("{}\n{}\n{}", error.title, error.detail, error.recovery)
    } else if model.message.starts_with("Job ") {
        safe(&model.message)
    } else if let Some((_, notice)) = settings
        .notice
        .as_ref()
        .filter(|(group, _)| *group == if settings.tab < 2 { 0 } else { settings.tab })
    {
        safe(notice)
    } else if let Some(warning) =
        &settings.load_warnings[if settings.tab < 2 { 0 } else { settings.tab }]
    {
        safe(warning)
    } else {
        match settings.tab {
            0 => " Changes apply after Save. No requests are sent here.".into(),
            1 => {
                if settings.api.settings.secrets_stored.is_empty() {
                    " No stored secret reference.".into()
                } else {
                    " Keychain reference saved; access not checked.".into()
                }
            }
            2 => " Existing jobs keep their configuration.".into(),
            _ => " Font changes apply to a new Ghostty window.".into(),
        }
    };
    message(frame, rows[3], note, if error.is_some() { RED } else { FG });
    let save_label = if settings.tab < 2 {
        " Save API + Secrets "
    } else {
        " Save "
    };
    let buttons = [(save_label, count), (" Back ", count + 1)]
        .into_iter()
        .map(|(label, i)| {
            Span::styled(
                label,
                if settings.focus == Some(i) {
                    motion::selected(model)
                } else {
                    Style::default().fg(GREEN)
                },
            )
        })
        .collect::<Vec<_>>();
    let mut buttons = buttons;
    if settings.dirty.iter().any(|value| *value) {
        buttons.push(Span::styled(" * unsaved", Style::default().fg(FG)));
    }
    frame.render_widget(Paragraph::new(Line::from(buttons)), rows[4]);
    frame.render_widget(
        Paragraph::new(if settings.tab < 2 {
            " Ctrl+S save profile · Esc back"
        } else {
            " Ctrl+S save tab · Esc back"
        })
        .style(Style::default().fg(FG)),
        rows[5],
    );
}

fn render_form(frame: &mut Frame, model: &Model) {
    let (title, fields, focus, cursor, hint, footer) = if let Some(f) = &model.form {
        (
            " API ",
            f.fields()
                .into_iter()
                .map(|(l, v)| (l.into(), v))
                .collect::<Vec<(String, String)>>(),
            f.focus,
            f.cursor(),
            "Tab: field · Space: remote",
            "Enter: save · Esc: back",
        )
    } else if let Some(f) = &model.thread_form {
        (
            " THREADS ",
            vec![(
                if f.job.is_some() {
                    format!("JOB · current {}", f.previous)
                } else {
                    format!("NEW JOBS · current {}", f.previous)
                },
                f.value.clone(),
            )],
            0,
            f.cursor,
            "Workers 1–256 · RPS preserved",
            "Enter: save · Esc: back",
        )
    } else if let Some(f) = &model.export_form {
        (
            " EXPORT RESULTS ",
            vec![
                (
                    "Format".into(),
                    if f.format_csv { "CSV" } else { "Parquet" }.into(),
                ),
                (
                    "Include uncertain results?".into(),
                    if f.partial {
                        "[x] PARTIAL"
                    } else {
                        "[ ] Confirmed results only"
                    }
                    .into(),
                ),
            ],
            f.focus,
            None,
            "Tab: option · Space: toggle",
            "Enter: export · Esc: back",
        )
    } else {
        let f = model.appearance_form.as_ref().unwrap();
        (
            " APPEARANCE / GHOSTTY ",
            vec![
                (
                    "Font".into(),
                    if f.font_vga { "IBM VGA 8x16" } else { "Menlo" }.into(),
                ),
                ("Size".into(), format!("{} pt", f.font_size)),
                (
                    "ANSI motion".into(),
                    ["OFF", "SLOW", "NORMAL", "FAST"][f.speed as usize].into(),
                ),
                ("Apply font".into(), "[ OPEN GHOSTTY WINDOW ]".into()),
            ],
            f.focus,
            None,
            "Tab: option · ←/→: adjust",
            if f.focus == 3 {
                "Enter: open Ghostty · Esc: back"
            } else {
                "Enter: save · Esc: back"
            },
        )
    };
    let border = block(title);
    let inner = border.inner(frame.area());
    frame.render_widget(border, frame.area());
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(if model.error.is_some() {
            if frame.area().width < 70 {
                5
            } else {
                3
            }
        } else {
            1
        }),
        Constraint::Length(1),
    ])
    .split(inner);
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().fg(MUTED)),
        rows[0],
    );
    let visible = (rows[1].height as usize / 3).max(1);
    let start = focus.saturating_sub(visible - 1);
    let mut y = rows[1].y;
    for (i, (label, value)) in fields.iter().enumerate().skip(start).take(visible) {
        let focused = i == focus;
        let width = rows[1].width.saturating_sub(3) as usize;
        let value = safe(value);
        let (value, col) = if focused {
            input_window(&value, cursor, width)
        } else {
            (clipped(&value, width), 0)
        };
        frame.render_widget(
            Paragraph::new(format!("{} {label}", if focused { ">" } else { " " }))
                .style(Style::default().fg(if focused { GREEN } else { FG })),
            Rect::new(rows[1].x, y, rows[1].width, 1),
        );
        frame.render_widget(
            Paragraph::new(format!("  {value}")).style(if focused {
                Style::default().fg(FG).bg(super::palette::SELECTED)
            } else {
                Style::default().fg(FG)
            }),
            Rect::new(rows[1].x, y + 1, rows[1].width, 1),
        );
        if focused && ((model.form.is_some() && focus < 4) || model.thread_form.is_some()) {
            frame.set_cursor_position((rows[1].x + 2 + col, y + 1));
        }
        y += 3;
    }
    let detail = if let Some(e) = &model.error {
        format!(
            "{}\n{}\n{}",
            e.title,
            e.detail.lines().next().unwrap_or(""),
            e.recovery
        )
    } else if model.appearance_form.is_some() {
        "Font/size apply to a new window.".into()
    } else if model.thread_form.is_some() {
        "Applies to the next run.".into()
    } else if model.export_form.is_some() {
        "UNKNOWN means an uncertain result.".into()
    } else {
        "Secret hidden in Keychain.".into()
    };
    frame.render_widget(
        Paragraph::new(detail)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(if model.error.is_some() { RED } else { MUTED })),
        rows[2],
    );
    frame.render_widget(
        Paragraph::new(footer).style(Style::default().fg(GREEN)),
        rows[3],
    );
}

fn render_help(frame: &mut Frame, model: &Model, area: Rect) {
    frame.render_widget(Clear, area);
    let title = format!(" HELP {}/4 · Tab · PgUp/Dn · Esc ", model.help_page % 4 + 1);
    panel(
        frame,
        &title,
        super::copy::HELP_PAGES[model.help_page % 4],
        area,
        model.scroll,
    );
}
