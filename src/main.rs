mod controller;
mod entity;
mod repository;
mod sync;
mod view;

use std::io;

use ratatui::prelude::*;

use crate::controller::app::AppController;

fn main() -> io::Result<()> {
    let mut app = AppController::new()?;

    let mut terminal = ratatui::init();
    terminal.clear()?;

    let app_result = app.run(terminal);
    ratatui::restore();
    app_result
}
