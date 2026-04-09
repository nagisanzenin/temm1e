//! Code block yank picker — numbered overlay for copying code blocks.
//!
//! Pressing `Ctrl+Y` opens this picker showing the most recent 9 code
//! blocks from the message history. User presses 1-9 to copy that
//! block to the clipboard via `arboard` (with an OSC 52 fallback for
//! headless / SSH terminals).

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};

use crate::app::AppState;

pub fn render_copy_picker(state: &AppState, area: Rect, buf: &mut Buffer) {
    if state.code_blocks.is_empty() {
        // No blocks — degenerate case, should be handled by caller
        return;
    }

    let popup_width = 82.min(area.width.saturating_sub(4));
    let popup_height = 18.min(area.height.saturating_sub(4));
    let popup = centered_rect(popup_width, popup_height, area);

    Clear.render(popup, buf);

    let block = Block::default()
        .title(" Yank Code Block ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(state.theme.accent);
    let inner = block.inner(popup);
    block.render(popup, buf);

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(""));

    // Most recent first: iter().rev().take(9) gives us #1..#9 newest-to-oldest
    for (i, cb) in state.code_blocks.iter().rev().take(9).enumerate() {
        let num = i + 1;
        let lang_display: String = if cb.lang.is_empty() {
            "text".to_string()
        } else {
            cb.lang.clone()
        };
        let preview: String = cb
            .text
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("")
            .chars()
            .take(48)
            .collect();

        lines.push(Line::from(vec![
            Span::styled(format!("  {num}. "), state.theme.accent),
            Span::styled(format!("[{:<8}] ", lang_display), state.theme.secondary),
            Span::styled(preview, state.theme.text),
            Span::styled(
                format!("  ({} lines)", cb.line_count),
                state.theme.secondary,
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press 1-9 to copy · Esc to cancel",
        state.theme.secondary,
    )));

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    para.render(inner, buf);
}

/// Attempt to copy text to the system clipboard. Falls back to OSC 52
/// escape sequence for headless / remote terminals where arboard fails.
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    match arboard::Clipboard::new() {
        Ok(mut cb) => cb.set_text(text.to_string()).map_err(|e| e.to_string()),
        Err(_) => write_osc52(text).map_err(|e| e.to_string()),
    }
}

fn write_osc52(text: &str) -> std::io::Result<()> {
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine as _;
    use std::io::Write;
    let encoded = B64.encode(text);
    let mut stdout = std::io::stdout();
    write!(stdout, "\x1b]52;c;{}\x07", encoded)?;
    stdout.flush()
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.left() + (area.width.saturating_sub(width)) / 2;
    let y = area.top() + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
