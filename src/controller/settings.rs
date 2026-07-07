use chrono::NaiveTime;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::controller::popover::PopoverScreen;
use crate::controller::screen::{AppAction, Screen};
use crate::entity::setting::{Setting, SettingType, SETTING_BEDTIME, SETTING_CAFFEINE_MG_AT_BEDTIME, SETTING_SYNC_REMOTE_URL};
use crate::repository::connection::DbConnection;
use crate::repository::setting::SettingRepository;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusedField {
    Bedtime,
    CaffeineMg,
    SyncRemoteUrl,
}

pub struct SettingsScreen {
    bedtime: String,
    caffeine_mg: String,
    sync_remote_url: String,
    bedtime_valid: bool,
    caffeine_mg_valid: bool,
    focused: FocusedField,
    bedtime_error: Option<String>,
    caffeine_mg_error: Option<String>,
    saved: bool,
}

impl SettingsScreen {
    pub fn new() -> Result<Self, duckdb::Error> {
        let db = DbConnection::open("cuppa.db")?;
        let repo = SettingRepository::new(db)?;

        let bedtime = repo
            .get_setting(SETTING_BEDTIME)?
            .map(|s| s.value)
            .unwrap_or_else(|| "23:00".to_string());

        let caffeine_mg = repo
            .get_setting(SETTING_CAFFEINE_MG_AT_BEDTIME)?
            .map(|s| s.value)
            .unwrap_or_else(|| "50".to_string());

        let sync_remote_url = repo
            .get_sync_remote_url()?
            .unwrap_or_default();

        let bedtime_valid = Self::validate_bedtime(&bedtime);
        let caffeine_mg_valid = Self::validate_caffeine_mg(&caffeine_mg);

        Ok(Self {
            bedtime,
            caffeine_mg,
            sync_remote_url,
            bedtime_valid,
            caffeine_mg_valid,
            focused: FocusedField::Bedtime,
            bedtime_error: None,
            caffeine_mg_error: None,
            saved: false,
        })
    }

    fn validate_bedtime(value: &str) -> bool {
        NaiveTime::parse_from_str(value, "%H:%M").is_ok()
    }

    fn validate_caffeine_mg(value: &str) -> bool {
        value.parse::<i32>().is_ok_and(|v| v >= 0)
    }

    fn validate_all(&mut self) -> bool {
        self.bedtime_valid = Self::validate_bedtime(&self.bedtime);
        self.caffeine_mg_valid = Self::validate_caffeine_mg(&self.caffeine_mg);

        self.bedtime_error = if self.bedtime_valid {
            None
        } else {
            Some("Invalid time. Use HH:MM (24-hour)".to_string())
        };

        self.caffeine_mg_error = if self.caffeine_mg_valid {
            None
        } else {
            Some("Invalid number. Must be >= 0".to_string())
        };

        self.bedtime_valid && self.caffeine_mg_valid
    }

    fn save(&mut self) -> Result<(), duckdb::Error> {
        if !self.validate_all() {
            return Ok(());
        }

        let db = DbConnection::open("cuppa.db")?;
        let repo = SettingRepository::new(db)?;

        let bedtime = NaiveTime::parse_from_str(&self.bedtime, "%H:%M").unwrap();
        repo.set_bedtime(bedtime)?;

        let caffeine_mg = self.caffeine_mg.parse::<i32>().unwrap();
        repo.set_caffeine_mg_at_bedtime(caffeine_mg)?;

        repo.set_sync_remote_url(&self.sync_remote_url)?;

        self.saved = true;
        Ok(())
    }

    fn cycle_focus(&mut self) {
        self.focused = match self.focused {
            FocusedField::Bedtime => FocusedField::CaffeineMg,
            FocusedField::CaffeineMg => FocusedField::SyncRemoteUrl,
            FocusedField::SyncRemoteUrl => FocusedField::Bedtime,
        };
    }

    fn handle_bedtime_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Backspace => {
                self.bedtime.pop();
                self.saved = false;
            }
            KeyCode::Char(c) => {
                if self.bedtime.len() < 5 && (c.is_ascii_digit() || c == ':') {
                    self.bedtime.push(c);
                    self.saved = false;
                }
            }
            _ => {}
        }
        self.bedtime_valid = Self::validate_bedtime(&self.bedtime);
        self.bedtime_error = if self.bedtime_valid {
            None
        } else {
            Some("Invalid time. Use HH:MM (24-hour)".to_string())
        };
    }

    fn handle_caffeine_mg_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Backspace => {
                self.caffeine_mg.pop();
                self.saved = false;
            }
            KeyCode::Char(c) => {
                if c.is_ascii_digit() && self.caffeine_mg.len() < 5 {
                    self.caffeine_mg.push(c);
                    self.saved = false;
                }
            }
            _ => {}
        }
        self.caffeine_mg_valid = Self::validate_caffeine_mg(&self.caffeine_mg);
        self.caffeine_mg_error = if self.caffeine_mg_valid {
            None
        } else {
            Some("Invalid number. Must be >= 0".to_string())
        };
    }

    fn handle_sync_url_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Backspace => {
                self.sync_remote_url.pop();
                self.saved = false;
            }
            KeyCode::Char(c) => {
                if self.sync_remote_url.len() < 120 {
                    self.sync_remote_url.push(c);
                    self.saved = false;
                }
            }
            _ => {}
        }
    }
}

impl Screen for SettingsScreen {
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let popup_width = 60u16.min(area.width.saturating_sub(4));
        let popup_height = 20u16.min(area.height.saturating_sub(4));

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
            .title("Settings")
            .border_style(Style::default().fg(Color::White));
        frame.render_widget(block.clone(), popup_area);

        let inner = popup_area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        });

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Bedtime input
                Constraint::Length(1), // Bedtime error
                Constraint::Length(3), // Caffeine mg input
                Constraint::Length(1), // Caffeine mg error
                Constraint::Length(3), // Sync remote URL input
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Saved message
                Constraint::Length(1), // Footer
            ])
            .split(inner);

        // Bedtime field
        let bedtime_focused = self.focused == FocusedField::Bedtime;
        let bedtime_border_color = if bedtime_focused {
            Color::Yellow
        } else if !self.bedtime_valid && !self.bedtime.is_empty() {
            Color::Red
        } else {
            Color::Gray
        };

        let bedtime_label = if bedtime_focused {
            format!("> Bedtime (HH:MM): {}", self.bedtime)
        } else {
            format!("  Bedtime (HH:MM): {}", self.bedtime)
        };

        let bedtime_widget = Paragraph::new(bedtime_label)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(bedtime_border_color)),
            )
            .style(Style::default().fg(if bedtime_focused { Color::Yellow } else { Color::White }));
        frame.render_widget(bedtime_widget, layout[0]);

        // Bedtime error
        if let Some(ref error) = self.bedtime_error {
            let error_widget = Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red));
            frame.render_widget(error_widget, layout[1]);
        }

        // Caffeine mg field
        let caffeine_focused = self.focused == FocusedField::CaffeineMg;
        let caffeine_border_color = if caffeine_focused {
            Color::Yellow
        } else if !self.caffeine_mg_valid && !self.caffeine_mg.is_empty() {
            Color::Red
        } else {
            Color::Gray
        };

        let caffeine_label = if caffeine_focused {
            format!("> Caffeine at bedtime (mg): {}", self.caffeine_mg)
        } else {
            format!("  Caffeine at bedtime (mg): {}", self.caffeine_mg)
        };

        let caffeine_widget = Paragraph::new(caffeine_label)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(caffeine_border_color)),
            )
            .style(Style::default().fg(if caffeine_focused { Color::Yellow } else { Color::White }));
        frame.render_widget(caffeine_widget, layout[2]);

        // Caffeine mg error
        if let Some(ref error) = self.caffeine_mg_error {
            let error_widget = Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red));
            frame.render_widget(error_widget, layout[3]);
        }

        // Sync remote URL field
        let sync_focused = self.focused == FocusedField::SyncRemoteUrl;
        let sync_border_color = if sync_focused {
            Color::Yellow
        } else {
            Color::Gray
        };

        let sync_display = if self.sync_remote_url.is_empty() {
            "(none)".to_string()
        } else {
            self.sync_remote_url.clone()
        };

        let sync_label = if sync_focused {
            format!("> Git sync URL: {}", sync_display)
        } else {
            format!("  Git sync URL: {}", sync_display)
        };

        let sync_widget = Paragraph::new(sync_label)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(sync_border_color)),
            )
            .style(Style::default().fg(if sync_focused { Color::Yellow } else { Color::White }));
        frame.render_widget(sync_widget, layout[4]);

        // Saved message
        if self.saved {
            let saved_widget = Paragraph::new("✓ Saved!")
                .style(Style::default().fg(Color::Green))
                .alignment(Alignment::Center);
            frame.render_widget(saved_widget, layout[6]);
        }

        // Footer
        let footer = Paragraph::new("<Tab> Switch field  <Esc>/<q> Save & Close")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(footer, layout[7]);
    }

    fn handle_input(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                if let Err(_) = self.save() {
                    // Ignore save errors on exit
                }
                return AppAction::PopScreen;
            }
            KeyCode::Tab => {
                self.cycle_focus();
            }
            KeyCode::Backspace => {
                match self.focused {
                    FocusedField::Bedtime => self.handle_bedtime_input(key),
                    FocusedField::CaffeineMg => self.handle_caffeine_mg_input(key),
                    FocusedField::SyncRemoteUrl => self.handle_sync_url_input(key),
                }
            }
            KeyCode::Char(c) => {
                match self.focused {
                    FocusedField::Bedtime => self.handle_bedtime_input(key),
                    FocusedField::CaffeineMg => self.handle_caffeine_mg_input(key),
                    FocusedField::SyncRemoteUrl => self.handle_sync_url_input(key),
                }
            }
            _ => {}
        }
        AppAction::Continue
    }
}
