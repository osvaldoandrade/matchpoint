//! A decorative payment chip: PP faces, green bevels and bounded circuit pulses.
use super::palette;
use ratatui::{buffer::Buffer, layout::Rect, style::Color};
const P: [&str; 5] = ["11110", "10001", "11110", "10000", "10000"];
pub(super) struct Chip<'a> {
    area: Rect,
    buffer: &'a mut Buffer,
    origin: (i32, i32),
    phase: u64,
    tilt: f32,
}
impl<'a> Chip<'a> {
    pub fn new(area: Rect, buffer: &'a mut Buffer, phase: u64, circuit: bool) -> Self {
        Self {
            area,
            buffer,
            origin: (
                i32::from(area.x) + (i32::from(area.width) - 36) / 2,
                i32::from(area.y) + (i32::from(area.height) - 12) / 2,
            ),
            phase,
            tilt: if circuit {
                0.0
            } else {
                (phase as f32 * 0.045).sin() * 0.22 + 0.10
            },
        }
    }
    pub fn draw(&mut self, circuit: bool) {
        self.traces(circuit);
        for layer in (1..=2).rev() {
            for y in 2..9 {
                for x in 5..31 {
                    self.pixel(
                        x + layer,
                        y + layer,
                        if layer == 1 { '▓' } else { '░' },
                        palette::depth(0.4 - layer as f32 * 0.1),
                    );
                }
            }
        }
        self.face();
        self.letters();
    }
    fn letters(&mut self) {
        for letter in 0..2 {
            for (y, row) in P.iter().enumerate() {
                for (x, _) in row.bytes().enumerate().filter(|(_, bit)| *bit == b'1') {
                    for offset in 0..2 {
                        self.pixel(
                            7 + letter * 12 + x as i32 * 2 + offset,
                            3 + y as i32,
                            '█',
                            palette::FACE[y],
                        );
                    }
                }
            }
        }
    }
    fn face(&mut self) {
        for y in 2..9 {
            for x in 5..31 {
                let (symbol, light) = if y == 2 {
                    ('▄', 1.0)
                } else if y == 8 {
                    ('▀', 0.5)
                } else if x == 5 || x == 30 {
                    ('█', 0.8)
                } else {
                    ('░', 0.2)
                };
                let color = if (x as u64 + self.phase / 4) % 26 < 2 {
                    palette::MINT
                } else {
                    palette::depth(light)
                };
                self.pixel(x, y, symbol, color);
            }
        }
    }
    fn traces(&mut self, circuit: bool) {
        for row in [3, 5, 7] {
            for x in 0..5 {
                self.pixel(x, row, '-', palette::depth(0.5));
            }
            for x in 31..36 {
                self.pixel(x, row, '-', palette::depth(0.5));
            }
            let tick = ((self.phase / 3 + row as u64) % 5) as i32;
            self.pixel(
                tick,
                row,
                if tick % 2 == 0 { '0' } else { '1' },
                palette::MINT,
            );
            self.pixel(
                31 + (4 - tick),
                row,
                if tick % 2 == 0 { '1' } else { '0' },
                palette::BRIGHT,
            );
        }
        for x in [9, 15, 21, 27] {
            self.pixel(x, 1, '|', palette::BRIGHT);
            self.pixel(x, 10, '|', palette::depth(0.5));
            self.pixel(x, 11, '+', palette::depth(0.5));
        }
        if circuit {
            for x in 0..36 {
                self.pixel(
                    x,
                    0,
                    if x % 6 == 0 { '+' } else { '.' },
                    palette::depth(0.3),
                );
            }
        }
    }
    fn pixel(&mut self, x: i32, y: i32, symbol: char, color: Color) {
        let x = self.origin.0 + x + ((y - 6) as f32 * self.tilt).round() as i32;
        let y = self.origin.1 + y;
        if x >= 0
            && y >= 0
            && x <= u16::MAX as i32
            && y <= u16::MAX as i32
            && self.area.contains((x as u16, y as u16).into())
        {
            self.buffer[(x as u16, y as u16)]
                .set_char(symbol)
                .set_fg(color)
                .set_bg(palette::BG);
        }
    }
}
