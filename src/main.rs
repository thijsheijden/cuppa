mod controller;
mod entity;
mod repository;
mod view;

use std::io;

use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    prelude::*,
};

use crate::controller::home::HomeController;
use crate::repository::connection::DbConnection;
use crate::view::home;

fn main() -> io::Result<()> {
    let db = DbConnection::open("cuppa.db").expect("Failed to open database");
    let controller = HomeController::new(db).expect("Failed to initialize controller");

    let mut terminal = ratatui::init();
    terminal.clear()?;

    let app_result = run(terminal, controller);
    ratatui::restore();
    app_result
}

fn run(mut terminal: Terminal<impl Backend>, controller: HomeController) -> io::Result<()> {
    loop {
        terminal.draw(|frame| {
            home::render(frame, &controller);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(());
            }
        }
    }
}
