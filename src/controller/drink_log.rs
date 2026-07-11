use std::cell::RefCell;
use std::rc::Rc;

use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use chrono::{Local, TimeZone, Utc};

use crate::controller::screen::{AppAction, Screen};
use crate::repository::drink::DrinkRepository;
use crate::repository::DrinkRecord;
use crate::sync::log::SyncLog;

const PAGE_SIZE: usize = 20;

pub struct DrinkLogScreen {
    drinks: Vec<DrinkRecord>,
    offset: usize,
    selected: usize,
    has_more: bool,
    confirming_delete: bool,
    sync_log: Rc<RefCell<SyncLog>>,
}

impl DrinkLogScreen {
    pub fn new(sync_log: Rc<RefCell<SyncLog>>) -> rusqlite::Result<Self> {
        let mut screen = Self {
            drinks: Vec::new(),
            offset: 0,
            selected: 0,
            has_more: true,
            confirming_delete: false,
            sync_log,
        };
        screen.load_more()?;
        Ok(screen)
    }

    fn load_more(&mut self) -> rusqlite::Result<()> {
        if !self.has_more {
            return Ok(());
        }

        let repo = DrinkRepository::with_sync_log(Rc::clone(&self.sync_log))?;

        let new_drinks = repo.get_drinks_paginated(self.offset, PAGE_SIZE)?;

        if new_drinks.len() < PAGE_SIZE {
            self.has_more = false;
        }

        self.drinks.extend(new_drinks);
        self.offset += PAGE_SIZE;
        Ok(())
    }

    fn select_next(&mut self) {
        if self.drinks.is_empty() {
            return;
        }
        if self.selected < self.drinks.len() - 1 {
            self.selected += 1;
            if self.selected >= self.drinks.len() - 2 && self.has_more {
                let _ = self.load_more();
            }
        }
    }

    fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    fn delete_selected(&mut self) -> rusqlite::Result<()> {
        if self.drinks.is_empty() || self.selected >= self.drinks.len() {
            return Ok(());
        }
        let drink = &self.drinks[self.selected];
        if let Some(id) = drink.id {
            let repo = DrinkRepository::with_sync_log(Rc::clone(&self.sync_log))?;
            repo.delete_drink(id)?;
            self.drinks.remove(self.selected);
            if self.selected >= self.drinks.len() && self.selected > 0 {
                self.selected -= 1;
            }
        }
        Ok(())
    }

    fn format_timestamp(dt: &chrono::DateTime<Utc>) -> String {
        let local = dt.with_timezone(&Local);
        let now = Local::now();
        let today = now.date_naive();
        let dt_date = local.date_naive();

        let time_str = local.format("%H:%M").to_string();

        if dt_date == today {
            format!("today at {}", time_str)
        } else if dt_date == today.pred_opt().unwrap_or(today) {
            format!("yesterday at {}", time_str)
        } else {
            format!("{} at {}", local.format("%a %d %b"), time_str)
        }
    }
}

impl Screen for DrinkLogScreen {
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        let view_area = centered_rect(60, 18, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Drink Log")
            .border_style(Style::default().fg(Color::White));
        frame.render_widget(block.clone(), view_area);

        let inner = view_area.inner(ratatui::layout::Margin {
            horizontal: 1,
            vertical: 1,
        });

        if self.drinks.is_empty() {
            let empty = Paragraph::new("No drinks logged yet.")
                .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(empty, inner);
            return;
        }

        let header = Row::new(vec![
            Cell::from(Span::styled("Time", Style::default().add_modifier(ratatui::style::Modifier::BOLD))),
            Cell::from(Span::styled("Drink", Style::default().add_modifier(ratatui::style::Modifier::BOLD))),
            Cell::from(Span::styled("Caffeine", Style::default().add_modifier(ratatui::style::Modifier::BOLD))),
        ]);

        let rows: Vec<Row> = self
            .drinks
            .iter()
            .enumerate()
            .map(|(i, drink)| {
                let style = if i == self.selected {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else {
                    Style::default()
                };
                Row::new(vec![
                    Cell::from(Self::format_timestamp(&drink.consumed_at)).style(style),
                    Cell::from(drink.drink_name.clone()).style(style),
                    Cell::from(format!("{} mg", drink.caffeine_mg)).style(style),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(22),
                Constraint::Min(20),
                Constraint::Length(10),
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::NONE));

        frame.render_widget(table, inner);

        // Footer hint
        let footer_area = Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(1),
            inner.width,
            1,
        );
        let footer_text = if self.confirming_delete {
            "Delete this drink? <Enter> Yes  <Esc> No"
        } else {
            "<j/k> Scroll  <d> Delete  <Esc> Back"
        };
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(footer, footer_area);

        // Confirmation overlay
        if self.confirming_delete {
            let popup_w = 36u16;
            let popup_h = 5u16;
            let popup = centered_rect(popup_w, popup_h, view_area);
            frame.render_widget(ratatui::widgets::Clear, popup);
            let confirm_block = Block::default()
                .borders(Borders::ALL)
                .title("Confirm Delete")
                .border_style(Style::default().fg(Color::Red));
            frame.render_widget(confirm_block, popup);

            let confirm_inner = popup.inner(ratatui::layout::Margin {
                horizontal: 1,
                vertical: 1,
            });
            let drink_name = self.drinks.get(self.selected)
                .map(|d| d.drink_name.as_str())
                .unwrap_or("");
            let msg = Paragraph::new(format!("Delete '{}' ?", drink_name))
                .alignment(ratatui::layout::Alignment::Center);
            frame.render_widget(msg, confirm_inner);
        }
    }

    fn handle_input(&mut self, key: KeyEvent) -> AppAction {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return AppAction::Quit;
        }

        if self.confirming_delete {
            match key.code {
                KeyCode::Enter | KeyCode::Char('y') => {
                    let _ = self.delete_selected();
                    self.confirming_delete = false;
                }
                KeyCode::Esc | KeyCode::Char('n') => {
                    self.confirming_delete = false;
                }
                _ => {}
            }
            return AppAction::Continue;
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.select_next();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.select_previous();
            }
            KeyCode::Char('d') | KeyCode::Backspace => {
                if !self.drinks.is_empty() {
                    self.confirming_delete = true;
                }
            }
            KeyCode::Esc => {
                return AppAction::PopScreen;
            }
            KeyCode::Char('q') => {
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
