use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::controller::screen::{AppAction, Screen};

pub struct ErrorScreen {
    message: String,
}

impl ErrorScreen {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl Screen for ErrorScreen {
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let popup_width = 60u16.min(area.width.saturating_sub(4));
        let popup_height = 10u16.min(area.height.saturating_sub(4));

        let horizontal = (area.width.saturating_sub(popup_width)) / 2;
        let vertical = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect::new(
            area.x + horizontal,
            area.y + vertical,
            popup_width,
            popup_height,
        );

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Error")
            .border_style(Style::default().fg(Color::Red));
        frame.render_widget(block.clone(), popup_area);

        let inner = popup_area.inner(ratatui::layout::Margin {
            horizontal: 1,
            vertical: 1,
        });

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(inner);

        let message_widget = Paragraph::new(self.message.clone())
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Red));
        frame.render_widget(message_widget, layout[0]);

        let footer = Line::from(vec![
            Span::styled("<Enter>", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::raw(" or "),
            Span::styled("<Esc>", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::raw(" to dismiss"),
        ]);
        let footer_widget = Paragraph::new(footer)
            .alignment(Alignment::Center);
        frame.render_widget(footer_widget, layout[1]);
    }

    fn handle_input(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => AppAction::PopScreen,
            _ => AppAction::Continue,
        }
    }
}
