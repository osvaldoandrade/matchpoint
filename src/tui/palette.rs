use ratatui::style::Color;

pub(crate) const BG: Color = Color::Rgb(13, 19, 26);
pub(crate) const SURFACE: Color = Color::Rgb(19, 28, 37);
pub(crate) const SELECTED: Color = Color::Rgb(32, 54, 54);
pub(crate) const FG: Color = Color::Rgb(207, 221, 227);
pub(crate) const MUTED: Color = Color::Rgb(139, 160, 175);
pub(crate) const BORDER: Color = Color::Rgb(57, 77, 88);
pub(crate) const TRACK: Color = Color::Rgb(51, 68, 77);
pub(crate) const BRIGHT: Color = Color::Rgb(97, 185, 160);
pub(crate) const MINT: Color = Color::Rgb(165, 221, 207);
pub(crate) const SHADOW: Color = Color::Rgb(31, 47, 45);
pub(crate) const QUEUE: Color = Color::Rgb(143, 170, 187);
pub(crate) const ACTIVE: Color = Color::Rgb(118, 189, 204);
pub(crate) const WARNING: Color = Color::Rgb(222, 184, 116);
pub(crate) const DANGER: Color = Color::Rgb(222, 145, 157);
pub(crate) const FACE: [Color; 5] = [
    MINT,
    Color::Rgb(136, 203, 187),
    Color::Rgb(113, 193, 170),
    BRIGHT,
    Color::Rgb(81, 147, 132),
];

pub(crate) fn depth(light: f32) -> Color {
    let ramp = [
        SHADOW,
        TRACK,
        Color::Rgb(59, 106, 95),
        FACE[4],
        BRIGHT,
        MINT,
    ];
    ramp[(light.clamp(0.0, 1.0) * 5.0) as usize]
}
