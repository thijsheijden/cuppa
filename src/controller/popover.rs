use ratatui::{
    layout::{Margin, Rect},
    style::Color,
    widgets::{Block, Borders, Clear},
    Frame,
};

use crate::controller::screen::{AppAction, Screen};

pub struct PopoverScreen {
    inner: Box<dyn Screen>,
    width: u16,
    height: u16,
}

impl PopoverScreen {
    pub fn new(inner: Box<dyn Screen>, width: u16, height: u16) -> Self {
        Self {
            inner,
            width,
            height,
        }
    }
}

impl Screen for PopoverScreen {
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Fill entire terminal with opaque background
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                frame.buffer_mut()[(x, y)].set_bg(Color::Black);
            }
        }

        let popup_area = centered_rect(self.width, self.height, area);
        frame.render_widget(Clear, popup_area);

        let inner_block = Block::default()
            .borders(Borders::ALL)
            .border_style(ratatui::style::Style::default().fg(Color::White));
        frame.render_widget(inner_block, popup_area);

        let _inner_area = popup_area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        });

        self.inner.render(frame);
    }

    fn handle_input(&mut self, key: ratatui::crossterm::event::KeyCode) -> AppAction {
        if key == ratatui::crossterm::event::KeyCode::Esc {
            return AppAction::PopScreen;
        }
        self.inner.handle_input(key)
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
