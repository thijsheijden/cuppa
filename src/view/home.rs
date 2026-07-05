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

    let daily_max = 400;
    let daily_pct = (controller.today_total_caffeine as f64 / daily_max as f64 * 100.0).min(100.0) as u32;

    let current_line = Line::from(vec![
        Span::styled("Current: ", Style::default().add_modifier(ratatui::style::Modifier::BOLD)),
        Span::raw(format!("{:.1} mg", controller.current_caffeine_level)),
    ]);
    let today_line = Line::from(vec![
        Span::styled("Today: ", Style::default().add_modifier(ratatui::style::Modifier::BOLD)),
        Span::raw(format!("{} mg ({}% of 400mg)", controller.today_total_caffeine, daily_pct)),
    ]);
    let sleep_line = match &controller.sleep_time {
        Some(time) => Line::from(vec![
            Span::styled("Sleep ready: ", Style::default().add_modifier(ratatui::style::Modifier::BOLD)),
            Span::raw(time.clone()),
        ]),
        None => Line::from(vec![
            Span::styled("Sleep ready: ", Style::default().add_modifier(ratatui::style::Modifier::BOLD)),
            Span::raw("now"),
        ]),
    };

    let stats_text = ratatui::text::Text::from(vec![current_line, today_line, sleep_line]);
    let stats_widget = Paragraph::new(stats_text)
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

    let recent_title = "Today's Drinks";
    let recent_table = Table::new(recent_rows, [Constraint::Length(8), Constraint::Min(0)])
        .block(Block::default().borders(Borders::ALL).title(recent_title))
        .header(
            Row::new(vec!["Time", "Drink"])
                .style(Style::default().add_modifier(ratatui::style::Modifier::BOLD)),
        );
    frame.render_widget(recent_table, bottom_layout[1]);

    let footer_text = Line::from(vec![
        Span::styled("<a>", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::raw(" Add  "),
        Span::styled("<l>", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::raw(" View Log  "),
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
        .max(50.0); // Ensure threshold is visible

    let data: Vec<(f64, f64)> = controller
        .caffeine_series
        .iter()
        .enumerate()
        .map(|(i, (_, level))| (i as f64, *level))
        .collect();

    let num_points = controller.caffeine_series.len() as f64;

    let x_labels: Vec<Line> = vec![
        Line::from(controller.caffeine_series.first().map(|(t, _)| t.as_str()).unwrap_or("-12h")),
        Line::from(controller.caffeine_series.get(controller.caffeine_series.len() / 4).map(|(t, _)| t.as_str()).unwrap_or("-6h")),
        Line::from(controller.caffeine_series.get(controller.caffeine_series.len() / 2).map(|(t, _)| t.as_str()).unwrap_or("now")),
        Line::from(controller.caffeine_series.get(3 * controller.caffeine_series.len() / 4).map(|(t, _)| t.as_str()).unwrap_or("+6h")),
        Line::from(controller.caffeine_series.last().map(|(t, _)| t.as_str()).unwrap_or("+12h")),
    ];

    let dataset = ratatui::widgets::Dataset::default()
        .marker(symbols::Marker::Dot)
        .graph_type(ratatui::widgets::GraphType::Line)
        .style(Style::default().fg(Color::Cyan))
        .data(&data);

    let threshold_data: Vec<(f64, f64)> = (0..controller.caffeine_series.len())
        .step_by(2)
        .map(|i| (i as f64, 50.0))
        .collect();
    let threshold_dataset = ratatui::widgets::Dataset::default()
        .marker(symbols::Marker::Dot)
        .graph_type(ratatui::widgets::GraphType::Scatter)
        .style(Style::default().fg(Color::Magenta))
        .data(&threshold_data);

    let chart = ratatui::widgets::Chart::new(vec![dataset, threshold_dataset])
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
        );

    frame.render_widget(chart, area);

    // Draw custom y-axis labels at precise positions inside the chart
    let inner = area.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    // Ratatui Chart reserves the bottom 2 rows of inner area:
    // - 1 row for the x-axis tick line
    // - 1 row for x-axis labels
    let plot_height = inner.height.saturating_sub(2) as f64;
    let plot_y = inner.y;
    let plot_bottom = plot_y + inner.height.saturating_sub(3);

    // 0 mg label at bottom of plot area (above x-axis tick line)
    let zero_label = Paragraph::new("0 mg")
        .style(Style::default().fg(Color::Gray));
    let zero_area = Rect::new(
        inner.x,
        plot_bottom,
        6,
        1,
    );
    frame.render_widget(zero_label, zero_area);

    // Max label at top of plot area
    let max_label_text = format!("{:.0} mg", max_level);
    let max_label = Paragraph::new(max_label_text.clone())
        .style(Style::default().fg(Color::Gray));
    let max_area = Rect::new(
        inner.x,
        plot_y,
        max_label_text.len() as u16 + 1,
        1,
    );
    frame.render_widget(max_label, max_area);

    // 50 mg label at precise height within plot area
    // Only show if there's room (at least 1 row away from 0 and max)
    if max_level > 50.0 {
        let threshold_y = ((plot_height - 1.0) * (1.0 - 50.0 / max_level)).round() as u16;
        let threshold_y = threshold_y.min(plot_height.round() as u16 - 1);
        // Ensure 50mg label doesn't overlap with 0mg or max labels
        let min_y = 1; // at least 1 row below max
        let max_y = plot_bottom.saturating_sub(plot_y).saturating_sub(2); // at least 2 rows above 0
        if threshold_y >= min_y && threshold_y <= max_y && max_y >= min_y {
            let label_50 = Paragraph::new("50 mg")
                .style(Style::default().fg(Color::Magenta));
            let label_50_area = Rect::new(
                inner.x,
                plot_y + threshold_y,
                6,
                1,
            );
            frame.render_widget(label_50, label_50_area);
        }
    }

    // 25% label
    let pct25 = max_level * 0.25;
    let y25 = ((plot_height - 1.0) * 0.75).round() as u16; // 75% from top = 25% from bottom
    let y25 = y25.min(plot_bottom.saturating_sub(plot_y).saturating_sub(1));
    if y25 >= 1 {
        let label_25 = Paragraph::new(format!("{:.0}", pct25))
            .style(Style::default().fg(Color::Gray));
        let label_25_area = Rect::new(
            inner.x,
            plot_y + y25,
            6,
            1,
        );
        frame.render_widget(label_25, label_25_area);
    }

    // 75% label
    let pct75 = max_level * 0.75;
    let y75 = ((plot_height - 1.0) * 0.25).round() as u16; // 25% from top = 75% from bottom
    let y75 = y75.min(plot_bottom.saturating_sub(plot_y).saturating_sub(1));
    if y75 >= 1 && y75 < y25 {
        let label_75 = Paragraph::new(format!("{:.0}", pct75))
            .style(Style::default().fg(Color::Gray));
        let label_75_area = Rect::new(
            inner.x,
            plot_y + y75,
            6,
            1,
        );
        frame.render_widget(label_75, label_75_area);
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
