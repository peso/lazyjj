pub mod bookmark_set_popup;
pub mod bookmarks_tab;
pub mod command_log_tab;
pub mod command_popup;
pub mod details_panel;
pub mod files_tab;
pub mod help_popup;
pub mod log_tab;
pub mod message_popup;
pub mod rebase_popup;
pub mod styles;
pub mod utils;

use crate::{
    ComponentInputResult,
    app::App,
    commander::{Commander, log::Head},
};
use anyhow::Result;
use ratatui::{
    Frame,
    crossterm::event::Event,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols,
};
use ratatui::{prelude::*, widgets::*};

pub enum ComponentAction {
    ViewFiles(Head),
    ViewLog(Head),
    ChangeHead(Head),
    SetPopup(Option<Box<dyn Component>>),
    Multiple(Vec<ComponentAction>),
    RefreshTab(),
}

pub trait Component {
    // Called when switching to tab
    fn focus(&mut self, _commander: &mut Commander) -> Result<()> {
        Ok(())
    }

    fn update(&mut self, _commander: &mut Commander) -> Result<Option<ComponentAction>> {
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()>;

    fn input(&mut self, commander: &mut Commander, event: Event) -> Result<ComponentInputResult>;
}

pub fn draw_app(f: &mut Frame, app: &mut App) -> Result<()> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(f.area());

    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    {
        // TODO change tab list when Command Log disabled
        let tabs = Tabs::new(
            app.tab_sequence
                .iter()
                .enumerate()
                .map(|(i, tab)| format!("[{}] {}", i + 1, tab)),
        )
        .block(
            Block::bordered()
                .title(" Tabs ")
                .border_type(BorderType::Rounded),
        )
        .highlight_style(Style::default().bg(app.env.config.highlight_color()))
        .select(
            app.tab_sequence
                .iter()
                .position(|tab| tab == &app.current_tab)
                .unwrap_or(0),
        )
        .divider(symbols::line::VERTICAL);

        f.render_widget(tabs, header_chunks[0]);
    }
    {
        let tab_keys = app
            .tab_sequence
            .iter()
            .enumerate()
            .map(|(i, _)| (i + 1).to_string())
            .collect::<Vec<String>>()
            .join("/");
        let tabs = Paragraph::new(format!(
            "q: quit | ?: help | R: refresh | {tab_keys}: change tab"
        ))
        .fg(Color::DarkGray)
        .block(
            Block::bordered()
                .title(" lazyjj ")
                .border_type(BorderType::Rounded)
                .fg(Color::default()),
        );

        f.render_widget(tabs, header_chunks[1]);
    }

    if let Some(current_tab) = app.get_current_tab() {
        current_tab.draw(f, chunks[1])?;
    }

    if let Some(popup) = app.popup.as_mut() {
        popup.draw(f, f.area())?;
    }

    Ok(())
}
