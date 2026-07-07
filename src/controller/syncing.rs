use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::controller::screen::{AppAction, Screen};
use ratatui::crossterm::event::{KeyCode, KeyEvent};

pub struct SyncingScreen;

impl SyncingScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Screen for SyncingScreen {
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        let popup = centered_rect(20, 5, area);

        frame.render_widget(Clear, popup);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));
        frame.render_widget(block, popup);

        let inner = popup.inner(ratatui::layout::Margin {
            horizontal: 1,
            vertical: 1,
        });

        let text = Paragraph::new("Syncing...")
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center);
        frame.render_widget(text, inner);
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
