use super::{block, safe, state_label, Model, FG, GREEN, RED, YELLOW};
use crate::tui::palette;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Paragraph, Wrap},
    Frame,
};

const RECORD_LABELS: [&str; 5] = ["Queue", "Active", "Done", "Failed", "Unknown"];
const RECORD_COLORS: [Color; 5] = [palette::QUEUE, palette::ACTIVE, GREEN, RED, YELLOW];
const JOB_LABELS: [&str; 4] = ["WAIT", "RUN", "DONE", "HOLD"];

#[derive(Default)]
struct Snapshot {
    records: [u128; 5],
    jobs: [u64; 4],
    batches: u128,
}
impl Snapshot {
    fn from_model(model: &Model) -> Self {
        let mut data = Self::default();
        for job in &model.jobs {
            let c = job.counts;
            let positive = |n: i64| n.max(0) as u128;
            let values = [
                positive(c.pending) + positive(c.retry_wait),
                positive(c.in_flight),
                positive(c.succeeded),
                positive(c.failed_final),
                positive(c.unknown),
            ];
            for (total, value) in data.records.iter_mut().zip(values) {
                *total += value;
            }
            data.batches += positive(job.batches);
            let group = match job.state.as_str() {
                "PENDING" | "PREPARING" | "PREPARED" => 0,
                "RUNNING" => 1,
                "COMPLETED" | "COMPLETED_WITH_ERRORS" => 2,
                _ => 3,
            };
            data.jobs[group] += 1;
        }
        data
    }
}

pub(super) fn render(frame: &mut Frame, model: &Model, area: Rect) {
    let data = Snapshot::from_model(model);
    if area.width < 76 {
        records(frame, &data, area, false);
        return;
    }
    let rows = if area.height >= 18 {
        Layout::vertical([Constraint::Min(10), Constraint::Length(6)])
            .spacing(1)
            .split(area)
    } else {
        Layout::vertical([Constraint::Percentage(100), Constraint::Length(0)]).split(area)
    };
    let top = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
        .spacing(1)
        .split(rows[0]);
    records(frame, &data, top[0], true);
    jobs(frame, &data, top[1]);
    if rows[1].height > 0 {
        let bottom = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
            .spacing(1)
            .split(rows[1]);
        batches(frame, model, &data, bottom[0]);
        activity(frame, model, bottom[1]);
    }
}

fn sizes(values: [u128; 5], width: usize) -> [usize; 5] {
    let total: u128 = values.iter().sum();
    if total == 0 {
        return [0; 5];
    }
    let mut widths = values.map(|value| (value * width as u128 / total) as usize);
    let mut remainder = values.map(|value| value * width as u128 % total);
    for _ in widths.iter().sum::<usize>()..width {
        let index = (0..5).max_by_key(|i| remainder[*i]).unwrap();
        widths[index] += 1;
        remainder[index] = 0;
    }
    widths
}

fn distribution(data: &Snapshot, width: usize) -> Line<'static> {
    let lengths = sizes(data.records, width);
    if data.records.iter().all(|value| *value == 0) {
        return Line::styled("░".repeat(width), Style::default().fg(palette::TRACK));
    }
    Line::from(
        lengths
            .into_iter()
            .zip(RECORD_COLORS)
            .map(|(n, color)| Span::styled("█".repeat(n), Style::default().fg(color)))
            .collect::<Vec<_>>(),
    )
}

fn completion(data: &Snapshot, width: usize) -> Line<'static> {
    let total: u128 = data.records.iter().sum();
    let done: u128 = data.records[2..].iter().sum();
    let filled = if total == 0 {
        0
    } else {
        (done * width as u128 / total) as usize
    };
    Line::from(vec![
        Span::styled("█".repeat(filled), Style::default().fg(GREEN)),
        Span::styled(
            "░".repeat(width - filled),
            Style::default().fg(palette::TRACK),
        ),
    ])
}

fn records(frame: &mut Frame, data: &Snapshot, area: Rect, bordered: bool) {
    let inner = if bordered {
        block("").inner(area)
    } else {
        area
    };
    let chart = Paragraph::new(record_lines(data, inner, bordered)).wrap(Wrap { trim: true });
    frame.render_widget(
        if bordered {
            chart.block(block(" RECORD FLOW ").style(Style::default().bg(palette::SURFACE).fg(FG)))
        } else {
            chart
        },
        area,
    );
}

fn record_lines(data: &Snapshot, inner: Rect, bordered: bool) -> Vec<Line<'static>> {
    let mut lines = vec![record_caption(data), completion(data, inner.width as usize)];
    if bordered && inner.height >= 8 {
        lines.push(Line::styled("RECORD MIX", Style::default().fg(FG)));
        lines.push(distribution(data, inner.width as usize));
    }
    lines.extend(record_legend(data));
    if inner.height >= 6 {
        lines.push(Line::styled(
            format!(
                "{} jobs · {} batches",
                data.jobs.iter().sum::<u64>(),
                data.batches
            ),
            Style::default().fg(FG),
        ));
    }
    lines
}

fn record_caption(data: &Snapshot) -> Line<'static> {
    let total: u128 = data.records.iter().sum();
    let processed: u128 = data.records[2..].iter().sum();
    Line::styled(
        if total == 0 {
            "No records yet · N: import CSV".into()
        } else {
            format!(
                "Processed {processed}/{total} · {}%",
                processed * 100 / total
            )
        },
        Style::default().fg(palette::MINT),
    )
}

fn record_legend(data: &Snapshot) -> Vec<Line<'static>> {
    [0..2, 2..5]
        .into_iter()
        .map(|range| {
            Line::from(
                range
                    .map(|i| {
                        Span::styled(
                            format!("{} {}  ", RECORD_LABELS[i], data.records[i]),
                            Style::default().fg(if data.records[i] == 0 {
                                FG
                            } else {
                                RECORD_COLORS[i]
                            }),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

fn jobs(frame: &mut Frame, data: &Snapshot, area: Rect) {
    let bars = data
        .jobs
        .iter()
        .enumerate()
        .map(|(i, value)| {
            Bar::default()
                .value(*value)
                .label(Line::from(JOB_LABELS[i]))
                .style(Style::default().fg(if i == 3 {
                    YELLOW
                } else {
                    RECORD_COLORS[i.min(2)]
                }))
                .value_style(Style::default().fg(palette::MINT))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        BarChart::default()
            .block(block(" JOB LIFECYCLE ").style(Style::default().bg(palette::SURFACE).fg(FG)))
            .data(BarGroup::default().bars(&bars))
            .bar_width(4)
            .bar_gap(2)
            .max(data.jobs.iter().copied().max().unwrap_or(0).max(1))
            .label_style(Style::default().fg(FG)),
        area,
    );
}

fn batch_color(batch: &crate::store::BatchInfo) -> Color {
    if batch.unknown > 0 {
        YELLOW
    } else if batch.failed > 0 {
        RED
    } else if batch.in_flight > 0 {
        palette::MINT
    } else if matches!(batch.state.as_str(), "COMPLETED" | "COMPLETED_WITH_ERRORS") {
        GREEN
    } else {
        palette::QUEUE
    }
}

fn batches(frame: &mut Frame, model: &Model, data: &Snapshot, area: Rect) {
    let width = area.width.saturating_sub(2) as usize;
    let tiles = model
        .batch_items
        .iter()
        .take(width / 2)
        .map(|batch| Span::styled("■ ", Style::default().fg(batch_color(batch))))
        .collect::<Vec<_>>();
    let mut lines = vec![Line::styled(
        format!("{} batches across {} jobs", data.batches, model.jobs.len()),
        Style::default().fg(palette::MINT),
    )];
    if tiles.is_empty() {
        lines.push(Line::from("Import CSV to create batches."));
    } else {
        lines.push(Line::from(tiles));
        lines.push(Line::from(format!(
            "{} visible · filter {}",
            model.batch_items.len().min(width / 2),
            safe(&model.filter)
        )));
    }
    lines.push(Line::from("Selected job · Jobs → Batches"));
    frame.render_widget(
        Paragraph::new(lines)
            .block(block(" BATCH MAP ").style(Style::default().bg(palette::SURFACE).fg(FG)))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn activity(frame: &mut Frame, model: &Model, area: Rect) {
    let mut lines = vec![Line::styled(
        if model.api_ready {
            "API setup enabled"
        } else {
            "API setup needed · S: Settings"
        },
        Style::default().fg(palette::MINT),
    )];
    if let Some(job) = model
        .jobs
        .iter()
        .find(|job| Some(&job.id) == model.selected_job.as_ref())
    {
        lines.push(Line::from(format!(
            "{} · {}",
            job.id.chars().take(8).collect::<String>(),
            state_label(&job.state)
        )));
    }
    lines.push(Line::from(safe(
        model
            .events
            .back()
            .map(String::as_str)
            .unwrap_or("Session ready"),
    )));
    frame.render_widget(
        Paragraph::new(lines)
            .block(block(" ACTIVITY ").style(Style::default().bg(palette::SURFACE).fg(FG)))
            .wrap(Wrap { trim: true }),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn segment_allocation_conserves_width_and_never_assigns_empty_states() {
        for values in [
            [0; 5],
            [42, 0, 8, 0, 3],
            [1, 1, 1, 1, 1],
            [u64::MAX as u128, 0, 1, 0, 0],
        ] {
            for width in [0, 1, 7, 34, 100] {
                let allocated = sizes(values, width);
                assert_eq!(
                    allocated.iter().sum::<usize>(),
                    if values == [0; 5] { 0 } else { width }
                );
                for i in 0..5 {
                    if values[i] == 0 {
                        assert_eq!(allocated[i], 0);
                    }
                }
            }
        }
    }
    #[test]
    fn snapshot_keeps_unknown_separate_and_accumulates_without_i64_overflow() {
        let mut model = Model::new();
        model.jobs.push(crate::engine::JobStatus {
            id: "synthetic".into(),
            state: "BLOCKED_UNKNOWN".into(),
            desired: "pause".into(),
            counts: crate::domain::Counts {
                pending: i64::MAX,
                retry_wait: i64::MAX,
                unknown: 3,
                failed_final: -1,
                ..Default::default()
            },
            batches: 7,
        });
        let data = Snapshot::from_model(&model);
        assert_eq!(data.records, [i64::MAX as u128 * 2, 0, 0, 0, 3]);
        assert_eq!(data.jobs, [0, 0, 0, 1]);
        assert_eq!(data.batches, 7);
    }
}
