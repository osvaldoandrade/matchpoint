//! Presentation effects use existing ticks and never delay input or replace text.
use super::{motion_phase, Model};
use crate::tui::palette;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

pub(crate) fn accent(model: &Model) -> Color {
    let age = model
        .interaction_at
        .map(|at| model.animation.wrapping_sub(at));
    let amount = if model.motion {
        age.filter(|age| *age < 8)
            .map(|age| (8 - age).pow(2))
            .unwrap_or(0)
    } else {
        0
    };
    let channel = |base: u8, highlight: u8| base + ((highlight - base) as u64 * amount / 64) as u8;
    Color::Rgb(channel(97, 165), channel(185, 221), channel(160, 207))
}

pub(crate) fn selected(model: &Model) -> Style {
    Style::default()
        .fg(accent(model))
        .bg(palette::SELECTED)
        .bold()
}

pub(crate) fn title(text: &str, model: &Model) -> Line<'static> {
    let head = (motion_phase(model) % 180) as i64 - 4;
    Line::from(
        text.chars()
            .enumerate()
            .map(|(i, ch)| {
                let distance = (i as i64 - head).abs();
                let color = if model.motion && distance < 3 {
                    if distance == 0 {
                        palette::MINT
                    } else {
                        palette::FACE[distance as usize]
                    }
                } else {
                    palette::FG
                };
                Span::styled(ch.to_string(), Style::default().fg(color).bold())
            })
            .collect::<Vec<_>>(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_feedback_settles_and_reduced_motion_has_no_transient() {
        let mut model = Model::new();
        model.animation = 100;
        model.interaction_at = Some(100);
        let highlight = accent(&model);
        model.animation = 104;
        let middle = accent(&model);
        model.animation = 108;
        assert_ne!(highlight, middle);
        assert_ne!(middle, accent(&model));
        assert_eq!(accent(&model), palette::BRIGHT);
        model.motion = false;
        for tick in [100, 104, 108, u64::MAX] {
            model.animation = tick;
            assert_eq!(accent(&model), palette::BRIGHT);
        }
        model.motion = true;
        model.animation = 2;
        model.interaction_at = Some(u64::MAX - 1);
        assert_ne!(accent(&model), palette::BRIGHT);
    }

    #[test]
    fn title_keeps_complete_unicode_text_at_every_phase() {
        let mut model = Model::new();
        for tick in 0..400 {
            model.animation = tick;
            let line = title("PartnerGetMember · API", &model);
            assert_eq!(
                line.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>(),
                "PartnerGetMember · API"
            );
        }
    }
}
