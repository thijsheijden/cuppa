mod controller;
mod entity;
mod paths;
mod repository;
mod sync;
mod view;

use std::io;

use ratatui::prelude::*;

use crate::controller::app::AppController;
use crate::paths::{db_path, sync_log_dir};

fn main() -> io::Result<()> {
    let sync_log_dir = sync_log_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get sync log directory"))?;

    let mut app = AppController::new(
        &db_path(),
        &sync_log_dir,
    )?;

    let mut terminal = ratatui::init();
    terminal.clear()?;

    let app_result = app.run(terminal);
    ratatui::restore();
    app_result
}
