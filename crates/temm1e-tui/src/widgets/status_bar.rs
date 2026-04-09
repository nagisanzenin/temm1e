//! Bottom status bar — 3-section layout.
//!
//! Left: session state indicator (idle / thinking / tool name / cancelled).
//! Center: model · provider · tokens · cost.
//! Right: context-window usage meter · git repo/branch.

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use temm1e_agent::agent_task_status::AgentTaskPhase;

use crate::app::{AppState, GitInfo};

/// Renders the status bar at the bottom of the screen.
pub struct StatusBar<'a> {
    state: &'a AppState,
}

impl<'a> StatusBar<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

impl<'a> Widget for StatusBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let style = self.state.theme.status_bar;
        let accent = self.state.theme.accent;
        let info = self.state.theme.info;
        let secondary = self.state.theme.secondary;
        let tool_running = self.state.theme.tool_running;
        let error = self.state.theme.error;

        // Fill background across the whole bar
        for x in area.left()..area.right() {
            buf[(x, area.top())].set_style(style);
        }

        // ── Left: state indicator ─────────────────────────
        let (symbol, label, sym_style) =
            state_indicator(self.state, accent, tool_running, secondary, error);
        let left_spans = vec![
            Span::styled(" ", style),
            Span::styled(symbol.to_string(), sym_style),
            Span::styled(" ", style),
            Span::styled(label, sym_style),
        ];
        let left_line = Line::from(left_spans);
        buf.set_line(area.left(), area.top(), &left_line, area.width);

        // ── Right: context meter + git info ───────────────
        let right_spans = right_section(self.state, info, secondary, accent, error);
        if !right_spans.is_empty() {
            let right_line = Line::from(right_spans).alignment(Alignment::Right);
            buf.set_line(area.left(), area.top(), &right_line, area.width);
        }

        // ── Center: model · provider · tokens · cost ──────
        let center_spans = center_section(self.state, style, accent, info, secondary);
        if !center_spans.is_empty() {
            // Center manually — compute the visible text length and offset
            let center_text_width: usize =
                center_spans.iter().map(|s| s.content.chars().count()).sum();
            let center_x = area
                .left()
                .saturating_add(area.width.saturating_sub(center_text_width as u16) / 2);
            let center_line = Line::from(center_spans);
            // Render center with a clipping width so it doesn't overwrite edges
            let clip_width = area
                .width
                .saturating_sub(center_x.saturating_sub(area.left()));
            buf.set_line(center_x, area.top(), &center_line, clip_width);
        }
    }
}

fn state_indicator(
    state: &AppState,
    accent: Style,
    tool_running: Style,
    secondary: Style,
    error: Style,
) -> (&'static str, String, Style) {
    // Idle when not working, regardless of last phase (except cancelled)
    if !state.is_agent_working {
        if matches!(
            state.activity_panel.phase,
            AgentTaskPhase::Interrupted { .. }
        ) {
            return ("⊗", "cancelled".to_string(), error);
        }
        return (
            "●",
            "idle".to_string(),
            secondary.add_modifier(Modifier::DIM),
        );
    }

    match &state.activity_panel.phase {
        AgentTaskPhase::Preparing | AgentTaskPhase::Classifying => {
            ("◐", "preparing".to_string(), accent)
        }
        AgentTaskPhase::CallingProvider { .. } => ("◐", "thinking".to_string(), accent),
        AgentTaskPhase::ExecutingTool { tool_name, .. } => {
            let truncated: String = tool_name.chars().take(12).collect();
            ("◉", format!("tool:{truncated}"), tool_running)
        }
        AgentTaskPhase::ToolCompleted { .. } => ("◐", "thinking".to_string(), accent),
        AgentTaskPhase::Finishing => ("⧖", "finishing".to_string(), accent),
        AgentTaskPhase::Done => (
            "●",
            "idle".to_string(),
            secondary.add_modifier(Modifier::DIM),
        ),
        AgentTaskPhase::Interrupted { .. } => ("⊗", "cancelled".to_string(), error),
    }
}

fn center_section(
    state: &AppState,
    style: Style,
    accent: Style,
    info: Style,
    secondary: Style,
) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let model = state.current_model.clone().unwrap_or_default();
    let provider = state.current_provider.clone().unwrap_or_default();

    if !model.is_empty() {
        spans.push(Span::styled(model, accent));
    }
    if !provider.is_empty() {
        if !spans.is_empty() {
            spans.push(Span::styled(" · ".to_string(), secondary));
        }
        spans.push(Span::styled(provider, style));
    }

    let ti = state.token_counter.total_input_tokens;
    let to = state.token_counter.total_output_tokens;
    if ti > 0 || to > 0 {
        if !spans.is_empty() {
            spans.push(Span::styled(" · ".to_string(), secondary));
        }
        spans.push(Span::styled(
            format!("{}in/{}out", format_tokens_u32(ti), format_tokens_u32(to)),
            info,
        ));
    }

    let cost = state.token_counter.total_cost_usd;
    if cost > 0.0 {
        if !spans.is_empty() {
            spans.push(Span::styled(" · ".to_string(), secondary));
        }
        spans.push(Span::styled(format!("${:.4}", cost), info));
    }

    spans
}

fn right_section(
    state: &AppState,
    _info: Style,
    secondary: Style,
    accent: Style,
    error: Style,
) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();

    // Context window meter (D5)
    if let Some(ref model) = state.current_model {
        use temm1e_core::types::model_registry::model_limits;
        let (ctx_window, _) = model_limits(model);
        if ctx_window > 0 {
            let used = state.token_counter.total_input_tokens as u64;
            let window = ctx_window as u64;
            let pct = ((used * 100) / window.max(1)).min(100);
            let filled = ((pct * 10) / 100) as usize;
            let meter: String = (0..10)
                .map(|i| if i < filled { '▓' } else { '░' })
                .collect();
            let meter_style = if pct >= 95 {
                error
            } else if pct >= 80 {
                error.add_modifier(Modifier::DIM)
            } else {
                secondary
            };
            spans.push(Span::styled(meter, meter_style));
            spans.push(Span::styled(format!(" {}% ", pct), meter_style));
        }
    }

    // Git repo + branch (A3)
    if let Some(ref git) = state.git_info {
        spans.push(Span::styled(git_repo_span(git), secondary));
        spans.push(Span::styled(git.repo_name.clone(), accent));
        spans.push(Span::styled(" · ".to_string(), secondary));
        spans.push(Span::styled(git.branch.clone(), secondary));
        spans.push(Span::styled(" ".to_string(), secondary));
    } else {
        // Pad a trailing space so right-alignment doesn't touch the border
        spans.push(Span::styled(" ".to_string(), secondary));
    }

    spans
}

fn git_repo_span(_git: &GitInfo) -> String {
    "▣ ".to_string()
}

fn format_tokens_u32(n: u32) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
