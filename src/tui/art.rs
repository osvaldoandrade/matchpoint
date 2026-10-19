//! Splash motifs; old serialized names remain readable as appearance preferences.
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SceneKind {
    #[default]
    Tesseract,
    #[serde(alias = "Torus")]
    Lattice,
}
impl SceneKind {
    pub fn next(self) -> Self {
        match self {
            Self::Lattice => Self::Tesseract,
            Self::Tesseract => Self::Lattice,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Tesseract => "Payment chip",
            Self::Lattice => "Circuit flow",
        }
    }
}
pub(super) struct SplashScene {
    pub phase: u64,
    pub motion: bool,
    pub kind: SceneKind,
}
impl Widget for SplashScene {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.width < 4 || area.height < 3 {
            return;
        }
        let phase = if self.motion { self.phase % 100_000 } else { 0 };
        let circuit = self.kind == SceneKind::Lattice;
        super::brand::Chip::new(area, buffer, phase, circuit).draw(circuit);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn motifs_animate_and_reduced_motion_is_fixed() {
        let area = Rect::new(0, 0, 70, 25);
        for kind in [SceneKind::Lattice, SceneKind::Tesseract] {
            let draw = |phase, motion| {
                let mut buffer = Buffer::empty(area);
                SplashScene {
                    phase,
                    motion,
                    kind,
                }
                .render(area, &mut buffer);
                buffer
            };
            assert_ne!(draw(0, true), draw(32, true));
            assert_eq!(draw(0, false), draw(32, false));
            assert!(draw(0, true)
                .content()
                .iter()
                .any(|cell| cell.symbol() == "█"));
        }
    }
    #[test]
    fn motif_painting_stays_inside_the_supplied_subrectangle() {
        let bounds = Rect::new(0, 0, 80, 30);
        for area in [
            Rect::new(5, 4, 40, 18),
            Rect::new(2, 2, 8, 4),
            Rect::new(0, 0, 0, 0),
        ] {
            for phase in [0, 32, 99_999] {
                let mut buffer = Buffer::empty(bounds);
                SplashScene {
                    phase,
                    motion: true,
                    kind: SceneKind::Tesseract,
                }
                .render(area, &mut buffer);
                for y in 0..bounds.height {
                    for x in 0..bounds.width {
                        if !area.contains((x, y).into()) {
                            assert_eq!(buffer[(x, y)].symbol(), " ");
                        }
                    }
                }
            }
        }
    }
}
