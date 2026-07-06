use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Widget},
    Frame,
};

use crate::controller::home::HomeController;

const VIEW_WIDTH: u16 = 87;
const VIEW_HEIGHT: u16 = 26;
const CHART_HEIGHT: u16 = 14;

pub fn render(frame: &mut Frame, controller: &HomeController) {
    let area = frame.area();

    let view_area = centered_rect(VIEW_WIDTH, VIEW_HEIGHT, area);

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(CHART_HEIGHT),
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

    let stats_text = ratatui::text::Text::from(vec![current_line, today_line]);
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
        Span::styled("<s>", Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)),
        Span::raw(" Settings  "),
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
        .max(50.0);

    // Fixed bar width of 2, with 40 bars we need 80 columns for bars
    // Plus 5 for y-axis labels, plus 2 for borders = 87 total
    let bar_width: u16 = 2;
    let num_bars = controller.caffeine_series.len() as u16;
    let needed_width = num_bars * bar_width;
    
    // Reserve left column for y-axis labels (wider to fit "mg")
    let label_width = 7u16;
    let chart_area = Rect::new(
        area.x + label_width,
        area.y,
        area.width.saturating_sub(label_width),
        area.height,
    );

    // Draw the block border first
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Caffeine Chart");
    frame.render_widget(block, chart_area);

    let inner = chart_area.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });

    // Plot area: reserve bottom 2 rows for x-axis labels + green dot
    let plot_height = inner.height.saturating_sub(2) as f64;
    let plot_y = inner.y;
    let plot_bottom = inner.y + inner.height.saturating_sub(3);
    let label_row_y = inner.y + inner.height - 2;
    let dot_row_y = inner.y + inner.height - 1;

    if plot_height <= 0.0 || inner.width == 0 {
        return;
    }

    let bar_set = ratatui::symbols::bar::NINE_LEVELS;
    let bar_style = Style::default().fg(Color::Yellow);
    let label_style = Style::default().fg(Color::Gray);

    // Draw dashed purple line at y=50 (caffeine level) before bars so bars show on top
    let line_y = if max_level >= 50.0 {
        let line_pct = 50.0 / max_level;
        let line_offset_from_bottom = (line_pct * plot_height).round() as u16;
        let line_y = plot_bottom.saturating_sub(line_offset_from_bottom);
        if line_y >= plot_y && line_y <= plot_bottom {
            Some(line_y)
        } else {
            None
        }
    } else {
        None
    };

    let mut x = inner.x;
    for (i, (time, level)) in controller.caffeine_series.iter().enumerate() {
        if x + bar_width > inner.x + inner.width {
            break;
        }

        let value = (*level).round() as u64;
        let max_val = max_level.round() as u64;
        let ticks = if max_val > 0 { value * 8 * plot_height as u64 / max_val } else { 0 };
        let full_cells = (ticks / 8) as u16;
        let partial = (ticks % 8) as usize;

        // Ensure we always leave at least one row of space above the highest bar
        let effective_full = full_cells.saturating_sub(1);
        let effective_partial = if full_cells > 0 { partial } else { 0 };

        // Draw bar from bottom up
        let bar_bottom = plot_bottom;
        for row in 0..plot_height as u16 {
            let y = bar_bottom - row;
            if row < effective_full {
                for col in 0..bar_width {
                    frame.buffer_mut().get_mut(x + col, y).set_symbol("█").set_style(bar_style);
                }
            } else if row == effective_full && effective_partial > 0 {
                let sym = match effective_partial {
                    1 => bar_set.one_eighth,
                    2 => bar_set.one_quarter,
                    3 => bar_set.three_eighths,
                    4 => bar_set.half,
                    5 => bar_set.five_eighths,
                    6 => bar_set.three_quarters,
                    7 => bar_set.seven_eighths,
                    _ => " ",
                };
                for col in 0..bar_width {
                    frame.buffer_mut().get_mut(x + col, y).set_symbol(sym).set_style(bar_style);
                }
            }
        }

        // Draw x-axis label every 4th bar (every hour)
        if i % 4 == 0 {
            let label = time.as_str();
            let label_len = label.len() as u16;
            let label_x = if label_len <= bar_width {
                x + (bar_width - label_len) / 2
            } else {
                x
            };
            if label_x + label_len <= inner.x + inner.width {
                for (j, ch) in label.chars().enumerate() {
                    let cx = label_x + j as u16;
                    if cx < inner.x + inner.width {
                        frame.buffer_mut().get_mut(cx, label_row_y).set_symbol(&ch.to_string()).set_style(label_style);
                    }
                }
            }
        }

        x += bar_width;
    }

    // Draw green dot below x-axis labels
    if let Some(current_time_idx) = controller.caffeine_series.iter().position(|(t, _)| {
        t == &controller.current_time
    }) {
        let dot_x = inner.x + (current_time_idx as u16 * bar_width) + (bar_width / 2);
        if dot_x < inner.x + inner.width {
            if dot_row_y < inner.y + inner.height {
                frame.buffer_mut().get_mut(dot_x, dot_row_y).set_symbol("●").set_style(Style::default().fg(Color::Green));
            }
        }
    }

    // Draw purple circle at bedtime x, y=bedtime_caffeine_mg if within bounds
    let threshold = controller.bedtime_caffeine_mg as f64;
    let threshold_pct = threshold / max_level;
    let threshold_offset_from_bottom = (threshold_pct * plot_height).round() as u16;
    let threshold_y = plot_bottom.saturating_sub(threshold_offset_from_bottom);
    if threshold_y >= plot_y && threshold_y <= plot_bottom {
        // Find the bar closest to bedtime
        let bedtime_parts: Vec<&str> = controller.bedtime.split(':').collect();
        if bedtime_parts.len() == 2 {
            if let (Ok(bedtime_h), Ok(bedtime_m)) = (bedtime_parts[0].parse::<u8>(), bedtime_parts[1].parse::<u8>()) {
                let mut closest_idx = None;
                let mut closest_diff = u16::MAX;
                let bedtime_minutes = (bedtime_h as u16) * 60 + (bedtime_m as u16);
                
                for (i, (t, _)) in controller.caffeine_series.iter().enumerate() {
                    let t_parts: Vec<&str> = t.split(':').collect();
                    if t_parts.len() == 2 {
                        if let (Ok(t_h), Ok(t_m)) = (t_parts[0].parse::<u8>(), t_parts[1].parse::<u8>()) {
                            let t_minutes = (t_h as u16) * 60 + (t_m as u16);
                            let diff = if t_minutes >= bedtime_minutes {
                                t_minutes - bedtime_minutes
                            } else {
                                bedtime_minutes - t_minutes
                            };
                            if diff < closest_diff {
                                closest_diff = diff;
                                closest_idx = Some(i);
                            }
                        }
                    }
                }
                
                if let Some(idx_bedtime) = closest_idx {
                    let circle_x = inner.x + (idx_bedtime as u16 * bar_width) + (bar_width / 2);
                    if circle_x < inner.x + inner.width {
                        frame.buffer_mut().get_mut(circle_x, threshold_y).set_symbol("●").set_style(Style::default().fg(Color::Magenta));
                    }
                }
            }
        }
    }

    // Draw y-axis labels in the left margin — 0, 25%, 50%, 75%, max with "mg" suffix
    let zero_label = Paragraph::new("0mg")
        .style(Style::default().fg(Color::Gray));
    let zero_area = Rect::new(area.x + 1, plot_bottom, label_width - 1, 1);
    frame.render_widget(zero_label, zero_area);

    let max_label_text = format!("{:.0}mg", max_level);
    let max_label = Paragraph::new(max_label_text.clone())
        .style(Style::default().fg(Color::Gray));
    let max_area = Rect::new(area.x + 1, plot_y, label_width - 1, 1);
    frame.render_widget(max_label, max_area);

    let pct25 = max_level * 0.25;
    let y25 = ((plot_height - 1.0) * 0.75).round() as u16;
    let y25 = y25.min(plot_bottom.saturating_sub(plot_y).saturating_sub(1));
    if y25 >= 1 {
        let label_25 = Paragraph::new(format!("{:.0}mg", pct25))
            .style(Style::default().fg(Color::Gray));
        let label_25_area = Rect::new(area.x + 1, plot_y + y25, label_width - 1, 1);
        frame.render_widget(label_25, label_25_area);
    }

    let pct50 = max_level * 0.50;
    let y50 = ((plot_height - 1.0) * 0.50).round() as u16;
    let y50 = y50.min(plot_bottom.saturating_sub(plot_y).saturating_sub(1));
    if y50 >= 1 && y50 != y25 {
        let label_50 = Paragraph::new(format!("{:.0}mg", pct50))
            .style(Style::default().fg(Color::Gray));
        let label_50_area = Rect::new(area.x + 1, plot_y + y50, label_width - 1, 1);
        frame.render_widget(label_50, label_50_area);
    }

    let pct75 = max_level * 0.75;
    let y75 = ((plot_height - 1.0) * 0.25).round() as u16;
    let y75 = y75.min(plot_bottom.saturating_sub(plot_y).saturating_sub(1));
    if y75 >= 1 && y75 != y50 && y75 != y25 {
        let label_75 = Paragraph::new(format!("{:.0}mg", pct75))
            .style(Style::default().fg(Color::Gray));
        let label_75_area = Rect::new(area.x + 1, plot_y + y75, label_width - 1, 1);
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
