use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::controller::home::HomeController;

const VIEW_WIDTH: u16 = 80;
const VIEW_HEIGHT: u16 = 24;

pub fn render(frame: &mut Frame, controller: &HomeController) {
    let area = frame.area();

    let view_area = centered_rect(VIEW_WIDTH, VIEW_HEIGHT, area);

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(view_area);

    let top = main_layout[0];
    let bottom = main_layout[1];
    let footer = main_layout[2];

    let top_widget = Paragraph::new("")
        .block(Block::default().borders(Borders::ALL).title("Caffeine Chart"));
    frame.render_widget(top_widget, top);

    let bottom_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(bottom);

    let stats_widget = Paragraph::new("")
        .block(Block::default().borders(Borders::ALL).title("Statistics"));
    frame.render_widget(stats_widget, bottom_layout[0]);

    let recent_rows: Vec<Row> = controller
        .todays_drinks
        .iter()
        .map(|(time, name)| {
            Row::new(vec![
                Cell::from(time.clone()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(name.clone()),
            ])
        })
        .collect();

    let recent_table = Table::new(recent_rows, [Constraint::Length(8), Constraint::Min(0)])
        .block(Block::default().borders(Borders::ALL).title("Today's Drinks"))
        .header(
            Row::new(vec!["Time", "Drink"])
                .style(Style::default().add_modifier(ratatui::style::Modifier::BOLD)),
        );
    frame.render_widget(recent_table, bottom_layout[1]);

    let footer_text = Line::from(vec![
        Span::styled("<q>", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::raw(" Quit"),
    ]);
    let footer_widget = Paragraph::new(footer_text)
        .alignment(Alignment::Center);
    frame.render_widget(footer_widget, footer);
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
