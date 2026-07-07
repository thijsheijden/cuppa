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
    let home = HomeController::new(db).expect("Failed to initialize controller");

    let mut app = AppController::new();
    app.push_screen(Box::new(home));

    let mut terminal = ratatui::init();
    terminal.clear()?;

    let app_result = app.run(terminal);
    ratatui::restore();
    app_result
}
