use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::controller::screen::{AppAction, Screen};
use ratatui::crossterm::event::{KeyCode, KeyEvent};

pub struct SyncingScreen {
    messages: Vec<String>,
}

impl SyncingScreen {
    pub fn new() -> Self {
        Self {
            messages: vec!["Syncing...".to_string()],
        }
    }

    pub fn with_messages(messages: Vec<String>) -> Self {
        Self { messages }
    }

    pub fn add_message(&mut self, msg: String) {
        self.messages.push(msg);
    }

    pub fn messages(&self) -> &[String] {
        &self.messages
    }
}

impl Screen for SyncingScreen {
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Dynamic height based on message count, min 5 max 20
        let msg_count = self.messages.len().max(1) as u16;
        let popup_height = (3 + msg_count).min(20).max(5);
        let popup_width = 50u16.min(area.width.saturating_sub(4));

        let popup = centered_rect(popup_width, popup_height, area);

        frame.render_widget(Clear, popup);

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Sync")
            .border_style(Style::default().fg(Color::White));
        frame.render_widget(block, popup);

        let inner = popup.inner(ratatui::layout::Margin {
            horizontal: 1,
            vertical: 1,
        });

        let constraints: Vec<Constraint> = self
            .messages
            .iter()
            .map(|_| Constraint::Length(1))
            .collect();

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        for (i, msg) in self.messages.iter().enumerate() {
            let style = if i == self.messages.len() - 1 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let text = Paragraph::new(msg.as_str())
                .style(style)
                .alignment(Alignment::Left);
            if i < layout.len() {
                frame.render_widget(text, layout[i]);
            }
        }
    }

    fn handle_input(&mut self, _key: KeyEvent) -> AppAction {
        AppAction::Continue
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let horizontal = (area.width.saturating_sub(width)) / 2;
    let vertical = (area.height.saturating_sub(height)) / 2;

    let width = width.min(area.width);
    let height = height.min(area.height);

    Rect::new(
        area.x + horizontal,
        area.y + vertical,
        width,
        height,
    )
}
