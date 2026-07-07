mod controller;
mod entity;
mod repository;
mod sync;
mod view;

use std::io;

use ratatui::prelude::*;

use crate::controller::app::AppController;
use crate::controller::home::HomeController;
use crate::controller::screen::Screen;
use crate::repository::connection::DbConnection;

fn main() -> io::Result<()> {
    let db = DbConnection::open("cuppa.db").expect("Failed to open database");
    let mut app = AppController::new().expect("Failed to initialize sync log");
    let sync_log = app.sync_log();
    let home = HomeController::new(db, sync_log).expect("Failed to initialize controller");

    app.push_screen(Box::new(home));

    let mut terminal = ratatui::init();
    terminal.clear()?;

    let app_result = app.run(terminal);
    ratatui::restore();
    app_result
}
