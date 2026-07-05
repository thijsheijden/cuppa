use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols,
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

    render_caffeine_chart(frame, top, controller);

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

fn render_caffeine_chart(frame: &mut Frame, area: Rect, controller: &HomeController) {
    if controller.caffeine_series.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Caffeine Chart");
        frame.render_widget(block, area);
        return;
    }

    let max_level = controller
        .caffeine_series
        .iter()
        .map(|(_, level)| *level)
        .fold(0.0f64, f64::max)
        .max(1.0);

    let data: Vec<(f64, f64)> = controller
        .caffeine_series
        .iter()
        .enumerate()
        .map(|(i, (_, level))| (i as f64, *level))
        .collect();

    let num_points = controller.caffeine_series.len() as f64;

    let x_labels = vec![
        Line::from("00:00"),
        Line::from("06:00"),
        Line::from("12:00"),
        Line::from("18:00"),
        Line::from("24:00"),
    ];

    let y_step = (max_level / 4.0).ceil();
    let y_labels: Vec<Line> = (0..=4)
        .map(|i| Line::from(format!("{:.0} mg", i as f64 * y_step)))
        .collect();

    let dataset = ratatui::widgets::Dataset::default()
        .marker(symbols::Marker::Dot)
        .graph_type(ratatui::widgets::GraphType::Line)
        .style(Style::default().fg(Color::Cyan))
        .data(&data);

    let chart = ratatui::widgets::Chart::new(vec![dataset])
        .block(Block::default().borders(Borders::ALL).title("Caffeine Chart"))
        .x_axis(
            ratatui::widgets::Axis::default()
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, num_points - 1.0])
                .labels(x_labels)
        )
        .y_axis(
            ratatui::widgets::Axis::default()
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, max_level])
                .labels(y_labels)
        );

    frame.render_widget(chart, area);
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
