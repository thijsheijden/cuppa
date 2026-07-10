use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::controller::error_screen::ErrorScreen;
use crate::controller::popover::PopoverScreen;
use crate::controller::screen::{AppAction, Screen};
use crate::repository::{
    connection::DbConnection,
    drink_type::DrinkTypeRepository,
};
use crate::paths::db_path;

const ESPRESSO_MG_PER_SHOT: i32 = 63;

enum CaffeineInputMode {
    Shots,
    Custom,
}

pub struct AddCustomDrinkScreen {
    name: String,
    name_focused: bool,
    shots: i32,
    custom_mg: String,
    custom_focused: bool,
    mode: CaffeineInputMode,
}

impl AddCustomDrinkScreen {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            name_focused: true,
            shots: 1,
            custom_mg: String::new(),
            custom_focused: false,
            mode: CaffeineInputMode::Shots,
        }
    }

    fn caffeine_mg(&self) -> i32 {
        match self.mode {
            CaffeineInputMode::Shots => self.shots * ESPRESSO_MG_PER_SHOT,
            CaffeineInputMode::Custom => self.custom_mg.parse().unwrap_or(0),
        }
    }

    fn save(&self) -> Result<(), duckdb::Error> {
        if self.name.trim().is_empty() {
            return Ok(());
        }
        let mg = self.caffeine_mg();
        if mg <= 0 {
            return Ok(());
        }

        let db = DbConnection::open(&db_path())?;
        let repo = DrinkTypeRepository::new(db)?;
        repo.add_custom_drink(&self.name, mg)?;
        Ok(())
    }

    fn cycle_mode(&mut self) {
        self.mode = match self.mode {
            CaffeineInputMode::Shots => CaffeineInputMode::Custom,
            CaffeineInputMode::Custom => CaffeineInputMode::Shots,
        };
    }
}

impl Screen for AddCustomDrinkScreen {
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        let popup = centered_rect(50, 14, area);

        frame.render_widget(Clear, popup);

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Add Custom Drink")
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

        // Name field
        let name_style = if self.name_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let name_block = Block::default()
            .borders(Borders::ALL)
            .title("Name")
            .border_style(name_style);
        let name_widget = Paragraph::new(self.name.as_str())
            .block(name_block)
            .style(name_style);
        frame.render_widget(name_widget, layout[0]);

        // Espresso shots field
        let shots_style = if matches!(self.mode, CaffeineInputMode::Shots) {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let shots_block = Block::default()
            .borders(Borders::ALL)
            .title(format!("Espresso Shots ({} mg each)", ESPRESSO_MG_PER_SHOT))
            .border_style(shots_style);
        let shots_text = format!("{} shot{} = {} mg", self.shots, if self.shots == 1 { "" } else { "s" }, self.shots * ESPRESSO_MG_PER_SHOT);
        let shots_widget = Paragraph::new(shots_text)
            .block(shots_block)
            .style(shots_style);
        frame.render_widget(shots_widget, layout[1]);

        // Custom mg field
        let custom_style = if matches!(self.mode, CaffeineInputMode::Custom) {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let custom_block = Block::default()
            .borders(Borders::ALL)
            .title("Custom Caffeine (mg)")
            .border_style(custom_style);
        let custom_widget = Paragraph::new(self.custom_mg.as_str())
            .block(custom_block)
            .style(custom_style);
        frame.render_widget(custom_widget, layout[2]);

        // Footer
        let footer = Line::from(vec![
            Span::styled("<Tab>", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::raw(" Switch mode  "),
            Span::styled("<Enter>", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::raw(" Save  "),
            Span::styled("<Esc>", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
            Span::raw(" Cancel"),
        ]);
        let footer_widget = Paragraph::new(footer)
            .alignment(Alignment::Center);
        frame.render_widget(footer_widget, layout[3]);
    }

    fn handle_input(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Tab => {
                if self.name_focused {
                    self.name_focused = false;
                    self.custom_focused = false;
                } else if matches!(self.mode, CaffeineInputMode::Shots) {
                    self.mode = CaffeineInputMode::Custom;
                    self.custom_focused = true;
                } else {
                    self.mode = CaffeineInputMode::Shots;
                    self.custom_focused = false;
                }
            }
            KeyCode::Char(c) if self.name_focused => {
                self.name.push(c);
            }
            KeyCode::Backspace if self.name_focused => {
                self.name.pop();
            }
            KeyCode::Char(c) if self.custom_focused && c.is_ascii_digit() => {
                self.custom_mg.push(c);
            }
            KeyCode::Backspace if self.custom_focused => {
                self.custom_mg.pop();
            }
            KeyCode::Left if matches!(self.mode, CaffeineInputMode::Shots) && !self.name_focused => {
                if self.shots > 1 {
                    self.shots -= 1;
                }
            }
            KeyCode::Right if matches!(self.mode, CaffeineInputMode::Shots) && !self.name_focused => {
                if self.shots < 10 {
                    self.shots += 1;
                }
            }
            KeyCode::Char('-') if matches!(self.mode, CaffeineInputMode::Shots) && !self.name_focused => {
                if self.shots > 1 {
                    self.shots -= 1;
                }
            }
            KeyCode::Char('+') | KeyCode::Char('=') if matches!(self.mode, CaffeineInputMode::Shots) && !self.name_focused => {
                if self.shots < 10 {
                    self.shots += 1;
                }
            }
            KeyCode::Enter => {
                if self.name_focused && !self.name.trim().is_empty() {
                    self.name_focused = false;
                } else {
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
