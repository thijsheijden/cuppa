use std::cell::RefCell;
use std::rc::Rc;

use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use chrono::Utc;

use crate::controller::error_screen::ErrorScreen;
use crate::controller::popover::PopoverScreen;
use crate::controller::screen::{AppAction, Screen};
use crate::controller::set_timestamp::SetTimestampScreen;
use crate::repository::{
    drink_type::DrinkTypeRepository,
    drink::DrinkRepository,
};
use crate::sync::log::SyncLog;

pub struct AddDrinkScreen {
    drink_types: Vec<(String, String, i32)>,
    filtered_types: Vec<(String, String, i32)>,
    search_query: String,
    search_focused: bool,
    list_state: RefCell<ListState>,
    sync_log: Rc<RefCell<SyncLog>>,
}

impl AddDrinkScreen {
    pub fn new(sync_log: Rc<RefCell<SyncLog>>) -> rusqlite::Result<Self> {
        let repo = DrinkTypeRepository::new()?;
        let drink_types = repo.get_drink_types_sorted_by_consumption()?;
        let filtered_types = drink_types.clone();

        let mut list_state = ListState::default();
        if !filtered_types.is_empty() {
            list_state.select(Some(0));
        }

        Ok(Self {
            drink_types,
            filtered_types,
            search_query: String::new(),
            search_focused: false,
            list_state: RefCell::new(list_state),
            sync_log,
        })
    }

    fn filter_types(&mut self) {
        let query = self.search_query.to_lowercase();
        self.filtered_types = self
            .drink_types
            .iter()
            .filter(|(_, name, _)| name.to_lowercase().contains(&query))
            .cloned()
            .collect();

        let mut state = self.list_state.borrow_mut();
        if !self.filtered_types.is_empty() {
            state.select(Some(0));
        } else {
            state.select(None);
        }
    }

    pub fn reload_drink_types(&mut self) -> rusqlite::Result<()> {
        let repo = DrinkTypeRepository::new()?;
        let new_types = repo.get_drink_types_sorted_by_consumption()?;
        if new_types.len() != self.drink_types.len() {
            self.drink_types = new_types;
            self.filter_types();
        } else {
            self.drink_types = new_types;
        }
        Ok(())
    }

    fn selected_index(&self) -> usize {
        self.list_state.borrow().selected().unwrap_or(0)
    }

    fn select_next(&self) {
        if self.filtered_types.is_empty() {
            return;
        }
        let i = self.selected_index();
        if i < self.filtered_types.len() - 1 {
            self.list_state.borrow_mut().select(Some(i + 1));
        }
    }

    fn select_previous(&self) {
        let i = self.selected_index();
        if i > 0 {
            self.list_state.borrow_mut().select(Some(i - 1));
        }
    }

    fn log_selected_drink(&self, consumed_at: chrono::DateTime<Utc>) -> rusqlite::Result<()> {
        if self.filtered_types.is_empty() {
            return Ok(());
        }
        let index = self.selected_index();
        let (_key, name, caffeine_mg) = &self.filtered_types[index];

        let repo = DrinkRepository::with_sync_log(Rc::clone(&self.sync_log))?;
        repo.add_drink(name, *caffeine_mg, consumed_at)?;
        Ok(())
    }

    fn log_selected_drink_now(&self) -> rusqlite::Result<()> {
        self.log_selected_drink(Utc::now())
    }

    fn handle_log_result(&self, result: rusqlite::Result<()>) -> AppAction {
        match result {
            Ok(()) => AppAction::PopScreen,
            Err(e) => {
                let error_screen = ErrorScreen::new(format!("Failed to log drink: {}", e));
                let popover = PopoverScreen::new(Box::new(error_screen), 50, 8);
                AppAction::PushScreen(Box::new(popover))
            }
        }
    }
}

impl Screen for AddDrinkScreen {
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let popup_width = 60u16.min(area.width.saturating_sub(4));
        let popup_height = 20u16.min(area.height.saturating_sub(4));

        let horizontal = (area.width.saturating_sub(popup_width)) / 2;
        let vertical = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = ratatui::layout::Rect::new(
            area.x + horizontal,
            area.y + vertical,
            popup_width,
            popup_height,
        );

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Add Drink")
            .border_style(Style::default().fg(Color::White));
        frame.render_widget(block.clone(), popup_area);

        let inner = popup_area.inner(ratatui::layout::Margin {
            horizontal: 1,
            vertical: 1,
        });

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(inner);

        let search_style = if self.search_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let search_text = if self.search_query.is_empty() && !self.search_focused {
            "Press / to search"
        } else {
            &self.search_query
        };

        let search_widget = Paragraph::new(search_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Search")
                    .border_style(search_style),
            )
            .style(search_style);
        frame.render_widget(search_widget, layout[0]);

        let items: Vec<ListItem> = self
            .filtered_types
            .iter()
            .map(|(_key, name, mg)| {
                ListItem::new(Line::from(vec![
                    Span::raw(format!("{} ", name)),
                    Span::styled(format!("({}mg)", mg), Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();

        let list_widget = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Drinks"))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list_widget, layout[1], &mut *self.list_state.borrow_mut());

        // Footer hint
        let footer_area = ratatui::layout::Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(1),
            inner.width,
            1,
        );
        let footer = Paragraph::new("</> Search  <a> Add Custom  <t> Set Time  <Enter> Log Now  <Esc> Cancel")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(footer, footer_area);
    }

    fn handle_input(&mut self, key: KeyEvent) -> AppAction {
        if self.search_focused {
            match key.code {
                KeyCode::Esc => {
                    self.search_focused = false;
                    if !self.filtered_types.is_empty() {
                        self.list_state.borrow_mut().select(Some(0));
                    }
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.filter_types();
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.filter_types();
                }
                KeyCode::Enter => {
                    self.search_focused = false;
                    let result = self.log_selected_drink_now();
                    return self.handle_log_result(result);
                }
                _ => {}
            }
            return AppAction::Continue;
        }

        match key.code {
            KeyCode::Char('/') => {
                self.search_focused = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.select_next();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.select_previous();
            }
            KeyCode::Enter => {
                let result = self.log_selected_drink_now();
                return self.handle_log_result(result);
            }
            KeyCode::Char('t') => {
                if self.filtered_types.is_empty() {
                    return AppAction::Continue;
                }
                let index = self.selected_index();
                let (_key, name, caffeine_mg) = self.filtered_types[index].clone();
                let timestamp_screen = SetTimestampScreen::new(
                    name,
                    caffeine_mg,
                    Rc::clone(&self.sync_log),
                );
                let popover = PopoverScreen::new(Box::new(timestamp_screen), 50, 14);
                return AppAction::PushScreen(Box::new(popover));
            }
            KeyCode::Char('a') => {
                let custom_screen = crate::controller::add_custom_drink::AddCustomDrinkScreen::new();
                let popover = PopoverScreen::new(Box::new(custom_screen), 50, 14);
                return AppAction::PushScreen(Box::new(popover));
            }
            KeyCode::F(5) => {
                let _ = self.reload_drink_types();
            }
            KeyCode::Esc => {
                return AppAction::PopScreen;
            }
            _ => {}
        }

        AppAction::Continue
    }
}
