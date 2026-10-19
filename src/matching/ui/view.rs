use super::{suggestions, Page, Ui};
use crate::tui::{
    palette as p,
    view::{
        block,
        form_field::{self, Focus},
        motion,
    },
};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Clear, Gauge, Paragraph, Wrap},
    Frame,
};

fn safe(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}
pub fn render(frame: &mut Frame, ui: &Ui) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().fg(p::FG).bg(p::BG)),
        area,
    );
    let border = block(" MATCHPOINT ");
    let inner = border.inner(area);
    frame.render_widget(border, area);
    if area.width < 40 || area.height < 18 {
        frame.render_widget(
            Paragraph::new(
                "Ultra-fast dataset matching\n\nResize to at least 40 × 18.\nCtrl+C quit",
            )
            .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }
    let message_width = Line::from(ui.message.as_str()).width();
    let status_height = if message_width > inner.width as usize {
        (message_width.div_ceil(inner.width as usize) + 2).min(6) as u16
    } else {
        2
    };
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(if !ui.signed_in && ui.page == Page::Home {
            1
        } else {
            3
        }),
        Constraint::Min(2),
        Constraint::Length(status_height),
        Constraint::Length(1),
    ])
    .split(inner);
    frame.render_widget(
        Paragraph::new(vec![
            motion::title(" Ultra-fast dataset matching", &ui.model),
            Line::styled(
                " CSV + Parquet / Local Filtering",
                Style::default().fg(p::MUTED),
            ),
        ]),
        rows[0],
    );
    if ui.page == Page::Help {
        ui.manual.render(
            frame,
            Rect::new(
                inner.x,
                inner.y + 2,
                inner.width,
                inner.height.saturating_sub(2),
            ),
        );
        return;
    }
    if !ui.signed_in && ui.page == Page::Home {
        frame.render_widget(
            Paragraph::new(" HOME / SIGN IN").style(Style::default().fg(p::MINT)),
            rows[1],
        );
    } else {
        menu(frame, ui, rows[1]);
    }
    match ui.page {
        Page::Home if !ui.signed_in => login(frame, ui, rows[2]),
        Page::Home => home(frame, ui, rows[2]),
        Page::Jobs => jobs(frame, ui, rows[2]),
        Page::NewJob | Page::Settings | Page::Export | Page::Threads => form(frame, ui, rows[2]),
        Page::Delete => delete_confirmation(frame, ui, rows[2]),
        Page::Help => unreachable!("Help uses the full-page manual"),
    }
    let live = ui.progress.as_ref().filter(|_| ui.busy);
    let now = std::time::Instant::now();
    let headline = live.map(|p| p.headline(now, area.width < 80));
    let status = if let Some(headline) = headline.as_deref().filter(|_| !ui.failed) {
        headline
    } else if ui.message.is_empty() && !ui.signed_in {
        "Sign in to unlock imports and jobs."
    } else if ui.message.is_empty() {
        match ui.page {
            Page::Home => [
                "New Job · Match a CSV or Parquet dataset against the loaded DAT.",
                "Jobs · Monitor, pause, resume and export local results.",
                "Settings · DAT source, key rules, secrets and appearance.",
            ][ui.model.home_selection],
            Page::NewJob => "Enter starts matching · Tab fields · ←→ changes transformation",
            Page::Settings => "←→ tabs · Tab fields · Ctrl+S save · Esc home",
            Page::Export => "Enter exports complete results · Existing files are preserved.",
            Page::Threads => "Enter saves worker count · Esc returns to Jobs.",
            Page::Delete => "Delete the selected stored job · Original and export files stay.",
            _ => "R run/resume · P pause · E export · K checksum",
        }
    } else {
        &ui.message
    };
    let detail = live.map(|p| p.detail(now));
    let second = if let Some(detail) = detail.as_deref() {
        detail
    } else if !ui.signed_in {
        if ui.busy {
            "Authenticating · Esc cancels · Ctrl+C quits"
        } else {
            "Secrets stay hidden · AES key stays local"
        }
    } else if ui.busy {
        "Operation active · P pause · Q stops safely"
    } else {
        "CSV / Parquet → local Filtering → key: true | false"
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                safe(status),
                Style::default().fg(if ui.failed { p::DANGER } else { p::MINT }),
            ),
            Line::styled(second, Style::default().fg(p::MUTED)),
        ])
        .wrap(Wrap { trim: false }),
        rows[3],
    );
    let footer = match (ui.page, area.width >= 80) {
        (Page::Home, true) if !ui.signed_in => {
            "Tab fields\tEnter continue\tF2 settings\tF1 help\tEsc quit"
        }
        (Page::Home, false) if !ui.signed_in => "Tab / Enter · F2 setup · Esc quit",
        (Page::Jobs, true) => "↑↓ jobs\tR run P pause\tT threads X delete\tE export K hash\tQ quit",
        (Page::Jobs, false) => "R run P pause T threads X delete E out",
        (Page::Delete, _) => "←→ select\tY delete\tEsc cancel",
        (Page::Threads, _) => "Enter save\tEsc jobs\tF1 help",
        (Page::NewJob, true) => "Tab fields\t←→ transform\tEnter run\tEsc home",
        (Page::NewJob, false) => "Tab fields  Enter run  Esc home",
        (Page::Settings, true) => "Tab fields\t←→ tabs/toggle\tCtrl+S save\tEsc home",
        (Page::Settings, false) => "Tab fields  Ctrl+S save  Esc home",
        (Page::Export, _) => "Enter export\tEsc cancel",
        (_, true) => "←→ select\tEnter open\t/ commands\tF1 help\tL sign out\tQ quit",
        (_, false) => "N new J jobs S setup F1 help Q quit",
    };
    frame.render_widget(
        Paragraph::new(expand_tabs(footer)).style(Style::default().fg(p::FG)),
        rows[4],
    );
    if let Some(input) = &ui.command {
        command(frame, ui, input, inner);
    }
}
fn expand_tabs(text: &str) -> String {
    let mut result = String::new();
    for (i, group) in text.split('\t').enumerate() {
        if i > 0 {
            let n = 8 - Line::from(result.as_str()).width() % 8;
            result.push_str(&" ".repeat(n));
        }
        result.push_str(group);
    }
    result
}
fn menu(frame: &mut Frame, ui: &Ui, area: Rect) {
    let border = block(" MENU ");
    let inner = border.inner(area);
    frame.render_widget(border, area);
    let columns = Layout::horizontal([Constraint::Ratio(1, 3); 3]).split(inner);
    let selected = match ui.page {
        Page::Home => Some(ui.model.home_selection),
        Page::NewJob => Some(0),
        Page::Jobs | Page::Export | Page::Threads | Page::Delete => Some(1),
        Page::Settings => Some(2),
        _ => None,
    };
    let labels = if area.width < 60 {
        ["N New Job", "J Jobs", "S Settings"]
    } else {
        ["[N] New Job", "[J] Jobs", "[S] Settings"]
    };
    for (i, label) in labels.into_iter().enumerate() {
        frame.render_widget(
            Paragraph::new(format!(
                "{}{label}",
                if selected == Some(i) { "›" } else { " " }
            ))
            .style(if !ui.signed_in && i < 2 {
                Style::default().fg(p::TRACK)
            } else if selected == Some(i) {
                motion::selected(&ui.model)
            } else {
                Style::default().fg(p::MUTED)
            }),
            columns[i],
        );
    }
}
fn login(frame: &mut Frame, ui: &Ui, area: Rect) {
    let width = area.width.min(76);
    let height = area.height.min(17);
    let area = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    let border = block(" SIGN IN ");
    let inner = border.inner(area);
    frame.render_widget(border, area);
    let rows = Layout::vertical([
        Constraint::Length(u16::from(inner.height >= 15)),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(inner);
    frame.render_widget(
        Paragraph::new(" Authenticate your data source to continue.")
            .style(Style::default().fg(p::MUTED)),
        rows[0],
    );
    let visible = (rows[1].height / 3).clamp(1, 4) as usize;
    let start = ui.focus.unwrap_or(0).min(3).saturating_sub(visible - 1);
    for (i, label) in [
        "Client ID",
        "Client secret · hidden",
        "Token URL · HTTPS",
        "DAT key · AES-256-GCM · Base64",
    ]
    .into_iter()
    .enumerate()
    .skip(start)
    .take(visible)
    {
        let value = if i == 1 || i == 3 {
            if ui.values[i].is_empty() {
                String::new()
            } else {
                "••••••••".into()
            }
        } else {
            safe(&ui.values[i])
        };
        form_field::render(
            frame,
            Rect::new(
                rows[1].x,
                rows[1].y + ((i - start) * 3) as u16,
                rows[1].width,
                3.min(rows[1].height),
            ),
            (label, &value),
            Focus {
                selected: ui.focus == Some(i) && !ui.busy,
                cursor: if i == 1 || i == 3 { None } else { ui.cursor },
                editable: !ui.busy,
                accent: p::MINT,
            },
        );
    }
    frame.render_widget(
        Paragraph::new(if ui.busy {
            "  Signing in…"
        } else {
            "  [ Sign in → ]"
        })
        .style(if ui.focus == Some(4) {
            motion::selected(&ui.model)
        } else {
            Style::default().fg(p::MINT)
        }),
        rows[2],
    );
}
fn home(frame: &mut Frame, ui: &Ui, area: Rect) {
    let show_recent = area.height >= 11;
    let rows = Layout::vertical([
        Constraint::Min(5),
        Constraint::Length(if show_recent { 5 } else { 0 }),
    ])
    .spacing(u16::from(show_recent))
    .split(area);
    let columns = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
        .spacing(1)
        .split(rows[0]);
    let total = ui.jobs.iter().map(|j| j.total as u128).sum::<u128>();
    let processed = ui.jobs.iter().map(|j| j.processed as u128).sum::<u128>();
    let yes = ui.jobs.iter().map(|j| j.positives as u128).sum::<u128>();
    let no = ui.jobs.iter().map(|j| j.negatives as u128).sum::<u128>();
    let border = block(" MATCH DISTRIBUTION ");
    let inner = border.inner(columns[0]);
    frame.render_widget(border, columns[0]);
    let lines = vec![
        Line::styled(
            format!("Processed {processed}/{total}"),
            Style::default().fg(p::MINT),
        ),
        graph(yes, total, inner.width, "true  ", p::MINT),
        graph(no, total, inner.width, "false ", p::ACTIVE),
        Line::styled(
            format!(
                "{} jobs · {} complete",
                ui.jobs.len(),
                ui.jobs.iter().filter(|j| j.state == "COMPLETED").count()
            ),
            Style::default().fg(p::MUTED),
        ),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
    let source = if let Some(snapshot) = &ui.snapshot {
        let info = snapshot.bloom.info();
        format!(
            "LOCKED IN RAM\n{:.1} KiB · {} keys\n{} probes · FPP {:.2}%\n{}\nSHA {}",
            info.bytes as f64 / 1024.0,
            info.inserted,
            info.hashes,
            info.target_fpp * 100.0,
            info.transform.label(),
            &info.sha256[..12]
        )
    } else if ui.config.url.is_empty() {
        "NO DAT SOURCE\nS → API: add URL\nD: load filter".into()
    } else {
        if ui.busy {
            "ACTIVE JOB OWNS DAT\nShared locked RAM\nReleased when job ends"
        } else {
            "DAT RELEASED\nNext job: fresh token + DAT\nD: preload next job"
        }
        .into()
    };
    frame.render_widget(
        Paragraph::new(source)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(p::MINT))
            .block(block(" FILTERING ")),
        columns[1],
    );
    let recent = ui
        .jobs
        .first()
        .map(|j| {
            format!(
                "{} · {}\ntrue {}   false {}   remaining {}",
                safe(&j.input_name),
                crate::tui::view::state_label(&j.state),
                j.positives,
                j.negatives,
                j.total.saturating_sub(j.processed)
            )
        })
        .unwrap_or_else(|| {
            "Choose a dataset in New Job.\nMatching runs locally; only the DAT is downloaded."
                .into()
        });
    frame.render_widget(
        Paragraph::new(recent)
            .wrap(Wrap { trim: true })
            .block(block(" LATEST DATASET ")),
        rows[1],
    );
}
fn graph(
    value: u128,
    total: u128,
    width: u16,
    label: &str,
    color: ratatui::style::Color,
) -> Line<'static> {
    let n = width.saturating_sub(17) as usize;
    let filled = if total == 0 {
        0
    } else {
        (value * n as u128 / total) as usize
    };
    Line::from(vec![
        Span::raw(label.to_string()),
        Span::styled("█".repeat(filled), Style::default().fg(color)),
        Span::styled("░".repeat(n - filled), Style::default().fg(p::TRACK)),
        Span::styled(format!(" {value}"), Style::default().fg(color)),
    ])
}
fn jobs(frame: &mut Frame, ui: &Ui, area: Rect) {
    let area = if let Some(progress) = ui.progress.as_ref().filter(|_| ui.busy) {
        let rows = Layout::vertical([Constraint::Length(5), Constraint::Min(0)]).split(area);
        activity(frame, progress, rows[0]);
        rows[1]
    } else {
        area
    };
    if area.height < 3 {
        return;
    }
    if ui.jobs.is_empty() {
        frame.render_widget(
            Paragraph::new("No datasets yet.\n\nN: create a job from CSV or Parquet.")
                .block(block(" JOBS ")),
            area,
        );
        return;
    }
    if area.height < 6
        || (area.width < 76
            && (area.height < 11
                || ui
                    .jobs
                    .get(ui.selected)
                    .is_some_and(|job| job.state == "COMPLETED")))
    {
        if let Some(job) = ui.jobs.get(ui.selected) {
            if let Some(path) = result_path(ui, job) {
                let title = format!(" RESULT · {} ", safe(&job.input_name));
                let border = block(&title);
                let inner = border.inner(area);
                frame.render_widget(border, area);
                frame.render_widget(
                    Paragraph::new(safe(&path.display().to_string())).wrap(Wrap { trim: false }),
                    inner,
                );
                return;
            }
            let border = block(" DATASET · ↑↓ ");
            let inner = border.inner(area);
            frame.render_widget(border, area);
            frame.render_widget(
                Paragraph::new(vec![
                    Line::raw(format!(
                        "{} · {}",
                        safe(&job.input_name),
                        crate::tui::view::state_label(&job.state)
                    )),
                    Line::styled(
                        format!(
                            "Saved {}/{} · {} threads",
                            job.processed, job.total, job.threads
                        ),
                        Style::default().fg(p::MUTED),
                    ),
                ]),
                inner,
            );
            if inner.height > 2 {
                frame.render_widget(
                    Gauge::default()
                        .ratio(if job.total == 0 {
                            0.0
                        } else {
                            (job.processed as f64 / job.total as f64).clamp(0.0, 1.0)
                        })
                        .gauge_style(Style::default().fg(p::BRIGHT).bg(p::TRACK))
                        .label(format!("{}/{}", job.processed, job.total)),
                    Rect::new(inner.x, inner.y + 2, inner.width, 1),
                );
            }
        }
        return;
    }
    let columns = if area.width >= 76 {
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
            .spacing(1)
            .split(area)
    } else {
        Layout::vertical([Constraint::Length(3), Constraint::Min(3)]).split(area)
    };
    let visible = columns[0].height.saturating_sub(2) as usize;
    let start = ui.selected.saturating_sub(visible.saturating_sub(1));
    let items = ui
        .jobs
        .iter()
        .enumerate()
        .skip(start)
        .take(visible.max(1))
        .map(|(i, j)| {
            Line::styled(
                format!(
                    "{} {} · {}",
                    if i == ui.selected { "›" } else { " " },
                    safe(&j.input_name),
                    crate::tui::view::state_label(&j.state)
                ),
                if i == ui.selected {
                    motion::selected(&ui.model)
                } else {
                    Style::default().fg(p::FG)
                },
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(items).block(block(" DATASETS · ↑↓ ")),
        columns[0],
    );
    let Some(job) = ui.jobs.get(ui.selected) else {
        return;
    };
    let border = block(" JOB DETAILS ");
    let inner = border.inner(columns[1]);
    frame.render_widget(border, columns[1]);
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(2),
    ])
    .split(inner);
    frame.render_widget(
        Paragraph::new(format!(
            "{} · {}\n{} · {} · {} threads",
            safe(&job.input_name),
            crate::tui::view::state_label(&job.state),
            safe(&job.rule.column),
            job.rule.transform.label(),
            job.threads
        )),
        rows[0],
    );
    frame.render_widget(
        Gauge::default()
            .ratio(if job.total == 0 {
                0.0
            } else {
                (job.processed as f64 / job.total as f64).clamp(0.0, 1.0)
            })
            .gauge_style(Style::default().fg(p::BRIGHT).bg(p::TRACK))
            .label(format!("{}/{}", job.processed, job.total)),
        rows[1],
    );
    let mut lines = Vec::new();
    if let Some(path) = result_path(ui, job) {
        lines.push(Line::raw(format!(
            "Result: {}",
            safe(&path.display().to_string())
        )));
    }
    lines.extend([
        graph(
            job.positives as u128,
            job.total as u128,
            rows[2].width,
            "true  ",
            p::MINT,
        ),
        graph(
            job.negatives as u128,
            job.total as u128,
            rows[2].width,
            "false ",
            p::ACTIVE,
        ),
        Line::styled(
            "true = possible match · FPP ≤ 0.1%",
            Style::default().fg(p::MUTED),
        ),
    ]);
    if job.state != "COMPLETED" {
        lines.push(Line::raw("R run/resume · P pause · E export · K checksum"));
    }
    if let Some(error) = &job.error {
        lines.push(Line::styled(safe(error), Style::default().fg(p::DANGER)));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), rows[2]);
}
fn result_path(ui: &Ui, job: &crate::matching::Job) -> Option<std::path::PathBuf> {
    if job.state != "COMPLETED" {
        return None;
    }
    if let Some(path) = &job.output_path {
        return path.is_absolute().then(|| path.clone());
    }
    ui.data_root
        .as_ref()
        .filter(|root| root.is_absolute())
        .map(|root| root.join("jobs").join(&job.id).join("result.txt"))
}
fn activity(frame: &mut Frame, progress: &crate::matching::Progress, area: Rect) {
    let title = progress.job.map_or_else(
        || " LIVE ACTIVITY ".into(),
        |id| format!(" LIVE ACTIVITY · {} ", &id.to_string()[..8]),
    );
    let border = block(&title);
    let inner = border.inner(area);
    frame.render_widget(border, area);
    let now = std::time::Instant::now();
    let rows = Layout::vertical([Constraint::Length(1); 3]).split(inner);
    frame.render_widget(
        Paragraph::new(progress.headline(now, inner.width < 76))
            .style(Style::default().fg(p::MINT)),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(progress.detail(now)).style(Style::default().fg(p::MUTED)),
        rows[1],
    );
    if progress.total.is_some_and(|n| n > 0)
        && !matches!(
            progress.phase,
            crate::matching::Phase::WaitingForDat | crate::matching::Phase::RetryWait
        )
    {
        frame.render_widget(
            Gauge::default()
                .ratio(progress.ratio())
                .gauge_style(Style::default().fg(p::BRIGHT).bg(p::TRACK))
                .label(format!("{:.1}%", progress.ratio() * 100.0)),
            rows[2],
        );
    } else {
        frame.render_widget(
            Paragraph::new("P pause · Q stop safely").style(Style::default().fg(p::FG)),
            rows[2],
        );
    }
}
fn form(frame: &mut Frame, ui: &Ui, area: Rect) {
    let (title, items) = match ui.page {
        Page::NewJob => (
            " NEW JOB ",
            vec![
                ("Dataset · CSV or Parquet", ui.values[0].clone(), true),
                ("Key column · name or #1", ui.values[1].clone(), true),
                ("Transformation", ui.rule.transform.label().into(), false),
                ("Matching threads · 1-256", ui.values[2].clone(), true),
            ],
        ),
        Page::Export => (
            " EXPORT RESULTS ",
            vec![("Output path", ui.values[0].clone(), true)],
        ),
        Page::Threads => (
            " MATCHING THREADS ",
            vec![("CPU workers · 1-256", ui.values[0].clone(), true)],
        ),
        _ => (" SETTINGS ", settings_fields(ui)),
    };
    let border = block(title);
    let inner = border.inner(area);
    frame.render_widget(border, area);
    let rows = Layout::vertical([
        Constraint::Length(u16::from(ui.page == Page::Settings)),
        Constraint::Min(1),
        Constraint::Length(u16::from(ui.page != Page::Export)),
    ])
    .split(inner);
    if ui.page == Page::Settings {
        let tabs = ["API", "Rules", "Secrets", "Appearance"]
            .into_iter()
            .enumerate()
            .map(|(i, label)| {
                Span::styled(
                    format!(" {label} "),
                    if i == ui.tab {
                        motion::selected(&ui.model)
                    } else {
                        Style::default().fg(p::MUTED)
                    },
                )
            })
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(Line::from(tabs)), rows[0]);
    }
    let visible = (rows[1].height / 3).max(1) as usize;
    let start = ui.focus.unwrap_or(0).saturating_sub(visible - 1);
    for (i, (label, value, editable)) in items.into_iter().enumerate().skip(start).take(visible) {
        let field = Rect::new(
            rows[1].x,
            rows[1].y + ((i - start) * 3) as u16,
            rows[1].width,
            3.min(rows[1].height),
        );
        form_field::render(
            frame,
            field,
            (label, &safe(&value)),
            Focus {
                selected: ui.focus == Some(i),
                cursor: ui.cursor,
                editable,
                accent: p::MINT,
            },
        );
    }
    if ui.page == Page::Export
        && rows[1].height > 3
        && std::path::Path::new(&ui.values[0]).is_absolute()
    {
        frame.render_widget(
            Paragraph::new(safe(&ui.values[0]))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(p::MUTED)),
            Rect::new(rows[1].x, rows[1].y + 3, rows[1].width, rows[1].height - 3),
        );
    }
    let hint = if ui.page == Page::NewJob {
        "Enter run · Esc home · F1 help"
    } else if ui.page == Page::Export {
        "Enter: export   Esc: cancel"
    } else if ui.page == Page::Threads {
        "Enter save · Esc jobs · F1 help"
    } else {
        "Tab fields · Ctrl+S save · F1 help"
    };
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().fg(p::MUTED)),
        rows[2],
    );
}
fn delete_confirmation(frame: &mut Frame, ui: &Ui, area: Rect) {
    let border = block(" DELETE JOB ");
    let inner = border.inner(area);
    frame.render_widget(border, area);
    if let Some(job) = ui.jobs.get(ui.selected) {
        let choice = if ui.focus == Some(1) {
            "  Cancel    [ DELETE ]"
        } else {
            "[ CANCEL ]    Delete"
        };
        let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
        let text=format!("{}\n{} · {} rows\nRemove stored input + results?\nOriginal/export files stay.\nCannot be undone.",safe(&job.input_name),&job.id[..8],job.total);
        frame.render_widget(
            Paragraph::new(text).style(Style::default().fg(p::DANGER)),
            rows[0],
        );
        frame.render_widget(
            Paragraph::new(choice).style(Style::default().fg(p::DANGER)),
            rows[1],
        );
    }
}
fn settings_fields(ui: &Ui) -> Vec<(&'static str, String, bool)> {
    match ui.tab {
        0 => vec![
            ("DAT URL · HTTPS", ui.values[0].clone(), true),
            (
                "Download request · arrows/Space",
                ui.config.download_mode.label().into(),
                false,
            ),
            ("DAT deadline · seconds", ui.values[5].clone(), true),
            ("DAT body idle · seconds", ui.values[6].clone(), true),
            ("Max download · MiB", ui.values[2].clone(), true),
            ("Range part size · MiB", ui.values[7].clone(), true),
            ("Parallel requests · 1-16", ui.values[8].clone(), true),
            ("Transient GET retries · 0-5", ui.values[10].clone(), true),
            ("Form fields · JSON, no secrets", ui.values[9].clone(), true),
            ("OAuth scope · optional", ui.values[1].clone(), true),
            ("OAuth deadline · seconds", ui.values[4].clone(), true),
            ("Max bit array · MiB", ui.values[3].clone(), true),
            (
                "Loopback HTTP · tests only",
                on(ui.config.allow_http_loopback),
                false,
            ),
        ],
        1 => vec![
            ("Default key column", ui.values[0].clone(), true),
            ("CSV delimiter", ui.values[1].clone(), true),
            ("Checkpoint · rows", ui.values[2].clone(), true),
            ("Matching threads · 1-256", ui.values[3].clone(), true),
            (
                "Transformation",
                ui.config.rule.transform.label().into(),
                false,
            ),
            (
                "Trim surrounding whitespace",
                on(ui.config.rule.trim),
                false,
            ),
            ("CSV has headers", on(ui.config.rule.headers), false),
        ],
        2 => vec![
            (
                "Session",
                if ui.signed_in {
                    "Signed in"
                } else {
                    "Sign in on Home"
                }
                .into(),
                false,
            ),
            (
                "Access token",
                if ui.signed_in {
                    "In memory · hidden"
                } else {
                    "Not issued · sign in on Home"
                }
                .into(),
                false,
            ),
            (
                "Token URL · create / refresh",
                ui.config.token_url.clone(),
                false,
            ),
            (
                "Refresh token",
                "Managed automatically · hidden".into(),
                false,
            ),
            ("Renew token", "Enter · refresh_token".into(), false),
        ],
        _ => vec![
            (
                "Font · dedicated Ghostty profile",
                if ui.appearance.font_vga {
                    "IBM VGA"
                } else {
                    "Menlo"
                }
                .into(),
                false,
            ),
            ("Font size", ui.appearance.font_size.to_string(), false),
            (
                "Motion · 0 disables",
                ui.appearance.speed.to_string(),
                false,
            ),
            (
                "Splash geometry",
                ui.appearance.scene.label().to_string(),
                false,
            ),
            (
                "Apply font",
                "Save, then reopen via Ghostty launcher".into(),
                false,
            ),
        ],
    }
}
fn on(value: bool) -> String {
    if value { "On" } else { "Off" }.into()
}
fn command(frame: &mut Frame, ui: &Ui, input: &str, area: Rect) {
    let options = suggestions(input);
    let h = (options.len() + 3).min(area.height as usize) as u16;
    let popup = Rect::new(area.x, area.bottom().saturating_sub(h), area.width, h);
    frame.render_widget(Clear, popup);
    let mut lines = vec![Line::styled(
        input.to_string(),
        Style::default().fg(p::MINT),
    )];
    lines.extend(options.iter().enumerate().map(|(i, text)| {
        Line::styled(
            format!("{} {text}", if i == ui.completion { "›" } else { " " }),
            if i == ui.completion {
                motion::selected(&ui.model)
            } else {
                Style::default().fg(p::FG)
            },
        )
    }));
    frame.render_widget(
        Paragraph::new(lines).block(block(" COMMANDS · Tab complete · Enter open · Esc close ")),
        popup,
    );
}
