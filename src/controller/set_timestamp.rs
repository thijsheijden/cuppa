use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use chrono::{Local, NaiveDateTime, TimeZone, Utc};

use crate::controller::error_screen::ErrorScreen;
use crate::controller::popover::PopoverScreen;
use crate::controller::screen::{AppAction, Screen};
use crate::repository::drink::DrinkRepository;
use crate::sync::log::SyncLog;

use std::cell::RefCell;
use std::rc::Rc;

enum Field {
    Date,
    Time,
}

pub struct SetTimestampScreen {
    date_input: String,
    time_input: String,
    focused_field: Field,
    drink_name: String,
    caffeine_mg: i32,
    sync_log: Rc<RefCell<SyncLog>>,
}

impl SetTimestampScreen {
    pub fn new(
        drink_name: String,
        caffeine_mg: i32,
        sync_log: Rc<RefCell<SyncLog>>,
    ) -> Self {
        let now = Local::now();
        let date_input = now.format("%Y-%m-%d").to_string();
        let time_input = now.format("%H:%M").to_string();

        Self {
            date_input,
            time_input,
            focused_field: Field::Date,
            drink_name,
            caffeine_mg,
            sync_log,
        }
    }

    fn parse_timestamp(&self) -> Option<chrono::DateTime<Utc>> {
        let combined = format!("{} {}", self.date_input, self.time_input);
        let naive = NaiveDateTime::parse_from_str(&combined, "%Y-%m-%d %H:%M").ok()?;
        Local.from_local_datetime(&naive).single().map(|dt| dt.with_timezone(&Utc))
    }

    fn save(&self) -> rusqlite::Result<()> {
        let consumed_at = match self.parse_timestamp() {
            Some(dt) => dt,
            None => return Ok(()),
        };

        let repo = DrinkRepository::with_sync_log(Rc::clone(&self.sync_log))?;
        repo.add_drink(&self.drink_name, self.caffeine_mg, consumed_at)?;
        Ok(())
    }

    fn handle_backspace(&mut self) {
        match self.focused_field {
            Field::Date => {
                self.date_input.pop();
            }
            Field::Time => {
                self.time_input.pop();
            }
        }
    }

    fn handle_char(&mut self, c: char) {
        match self.focused_field {
            Field::Date => {
                if c.is_ascii_digit() || c == '-' {
                    self.date_input.push(c);
                }
            }
            Field::Time => {
                if c.is_ascii_digit() || c == ':' {
                    self.time_input.push(c);
                }
            }
        }
    }

    fn cycle_field(&mut self) {
        self.focused_field = match self.focused_field {
            Field::Date => Field::Time,
            Field::Time => Field::Date,
        };
    }

    fn is_valid(&self) -> bool {
        self.parse_timestamp().is_some()
    }
}

impl Screen for SetTimestampScreen {
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        let popup = centered_rect(50, 14, area);

        frame.render_widget(Clear, popup);

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Set Timestamp")
            .border_style(Style::default().fg(Color::White));
        frame.render_widget(block, popup);

        let inner = popup.inner(ratatui::layout::Margin {
            horizontal: 1,
            vertical: 1,
        });

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(inner);

        // Drink info
        let info = Paragraph::new(format!("{} ({} mg)", self.drink_name, self.caffeine_mg))
            .alignment(Alignment::Center);
        frame.render_widget(info, layout[0]);

        // Date field
        let date_style = if matches!(self.focused_field, Field::Date) {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let date_block = Block::default()
            .borders(Borders::ALL)
            .title("Date (YYYY-MM-DD)")
            .border_style(date_style);
        let date_widget = Paragraph::new(self.date_input.as_str())
            .block(date_block)
            .style(date_style);
        frame.render_widget(date_widget, layout[1]);

        // Time field
        let time_style = if matches!(self.focused_field, Field::Time) {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let time_block = Block::default()
            .borders(Borders::ALL)
            .title("Time (HH:MM)")
            .border_style(time_style);
        let time_widget = Paragraph::new(self.time_input.as_str())
            .block(time_block)
            .style(time_style);
        frame.render_widget(time_widget, layout[2]);

        // Footer
        let footer = Line::from(vec![
            Span::styled(
                "<Tab>",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::raw(" Switch field  "),
            Span::styled(
                "<Enter>",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::raw(" Save  "),
            Span::styled(
                "<Esc>",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::raw(" Cancel"),
        ]);
        let footer_widget = Paragraph::new(footer).alignment(Alignment::Center);
        frame.render_widget(footer_widget, layout[3]);
    }

    fn handle_input(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Tab => {
                self.cycle_field();
            }
            KeyCode::Backspace => {
                self.handle_backspace();
            }
            KeyCode::Char(c) => {
                self.handle_char(c);
            }
            KeyCode::Enter => {
                if !self.is_valid() {
                    let error_screen = ErrorScreen::new(
                        "Invalid timestamp. Use YYYY-MM-DD and HH:MM.".to_string(),
                    );
                    let popover = PopoverScreen::new(Box::new(error_screen), 50, 8);
                    return AppAction::PushScreen(Box::new(popover));
                }
                let result = self.save();
                match result {
                    Ok(()) => return AppAction::PopScreen,
                    Err(e) => {
                        let error_screen = ErrorScreen::new(format!("Failed to save: {}", e));
                        let popover = PopoverScreen::new(Box::new(error_screen), 50, 8);
                        return AppAction::PushScreen(Box::new(popover));
                    }
                }
            }
            KeyCode::Esc => {
                return AppAction::PopScreen;
            }
            _ => {}
        }
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
