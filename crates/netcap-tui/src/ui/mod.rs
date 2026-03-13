pub mod detail_view;
pub mod request_list;
pub mod status_bar;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use crate::app::{App, AppTab};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(f.area());

    match app.tab {
        AppTab::RequestList => {
            request_list::draw(f, app, chunks[0]);
        }
        AppTab::Detail => {
            detail_view::draw(f, app, chunks[0]);
        }
    }

    status_bar::draw(f, app, chunks[1]);
}
