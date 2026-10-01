//! Terminal rendering of the bundled PNG; no terminal graphics protocol required.
use std::{io::Cursor, sync::OnceLock};

use ratatui::{layout::Rect, style::Color, widgets::Paragraph};

const PNG: &[u8] = include_bytes!("../assets/brand/drudwyn-white.png");

fn silhouette(size: usize) -> Option<String> {
    let mut reader = png::Decoder::new(Cursor::new(PNG)).read_info().ok()?;
    let mut bytes = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut bytes).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    let (width, height) = (info.width as usize, info.height as usize);
    let mut pixels = vec![vec![false; size]; size];
    // Area coverage preserves the silhouette better than point sampling at this size.
    for (y, row) in pixels.iter_mut().enumerate() {
        for (x, pixel) in row.iter_mut().enumerate() {
            let mut alpha = 0u64;
            let mut count = 0u64;
            for sy in y * height / size..(y + 1) * height / size {
                for sx in x * width / size..(x + 1) * width / size {
                    alpha += u64::from(bytes[(sy * width + sx) * 4 + 3]);
                    count += 1;
                }
            }
            *pixel = count > 0 && alpha > count * 127;
        }
    }
    let mut text = String::new();
    for y in (0..size).step_by(2) {
        for x in 0..size {
            text.push(match (pixels[y][x], pixels[y + 1][x]) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                _ => ' ',
            });
        }
        text.push('\n');
    }
    Some(text)
}

pub(crate) fn render(frame: &mut ratatui::Frame<'_>, area: Rect, background: Color) {
    static ICON: OnceLock<Option<String>> = OnceLock::new();
    static COMPACT: OnceLock<Option<String>> = OnceLock::new();
    let icon = if area.width < 16 || area.height < 8 {
        COMPACT.get_or_init(|| silhouette(8))
    } else {
        ICON.get_or_init(|| silhouette(16))
    };
    if let Some(icon) = icon {
        frame.render_widget(
            Paragraph::new(icon.as_str()).style(
                ratatui::style::Style::default()
                    .fg(Color::White)
                    .bg(background),
            ),
            area,
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn bundled_png_renders_a_nonempty_transparent_silhouette() {
        let icon = super::silhouette(16).expect("bundled RGBA PNG must decode");
        assert_eq!(icon.lines().count(), 8);
        assert!(icon.lines().all(|line| line.chars().count() == 16));
        assert!(icon.contains('█'));
        assert!(icon.contains(' '));
        assert!(icon.contains('▀') || icon.contains('▄'));
    }
}
