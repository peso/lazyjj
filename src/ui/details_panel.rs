use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Margin, Rect},
    text::{Line, Text},
    widgets::{
        Block, BorderType, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Wrap,
    },
};

/// Details panel used for the right side of each tab.
/// This handles scrolling and wrapping.
pub struct DetailsPanel {
    scroll: u16,
    height: u16,
    lines: u16,
    /// Line where drag motion started
    drag_origin: f32,
    wrap: bool,
}

/// Transient object holding render data
pub struct DetailsPanelRenderContext<'a> {
    panel: &'a mut DetailsPanel,
    title: Option<Line<'a>>,
    content: Option<Text<'a>>,
}

/// Commands that can be handled by the details panel
pub enum DetailsPanelEvent {
    ScrollDown,
    ScrollUp,
    ScrollDownHalfPage,
    ScrollUpHalfPage,
    ScrollDownPage,
    ScrollUpPage,
    DragBegin(/* rel_line */ f32),
    DragUpdate(/* rel_line */ f32),
    DragEnd(/* rel_line */ f32),
    ToggleWrap,
}

impl<'a> DetailsPanelRenderContext<'a> {
    pub fn new(panel: &'a mut DetailsPanel) -> Self {
        Self {
            panel,
            title: None,
            content: None,
        }
    }
    /// Set the title on the frame that surrounds the content
    pub fn title<T>(&mut self, title: T) -> &mut Self
    where
        T: Into<Line<'a>>,
    {
        self.title = Some(title.into());
        self
    }
    /// Set the text inside the panel
    pub fn content<T>(&mut self, content: T) -> &mut Self
    where
        T: Into<Text<'a>>,
    {
        self.content = Some(content.into());
        self
    }

    pub fn draw(&mut self, f: &mut ratatui::prelude::Frame<'_>, area: ratatui::prelude::Rect) {
        // Define border block
        let mut border = Block::bordered()
            .border_type(BorderType::Rounded)
            .padding(Padding::horizontal(1));
        // Apply title if provided
        if let Some(title) = &self.title {
            border = border.title_top(title.clone());
        }

        // Find text inside border
        let content_text = match &self.content {
            Some(text) => text,
            None => &Text::raw(""),
        };
        // Create content widget that uses border
        let paragraph_area = border.inner(area);
        let paragraph = self
            .panel
            .render(content_text.clone(), paragraph_area)
            .block(border);

        // render content and border
        f.render_widget(paragraph, area);

        // render file context on top of first line
        render_file_context(f, content_text, self.panel.scroll.into(), paragraph_area);

        // render scrollbar on top of border
        if self.panel.lines > paragraph_area.height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);

            let mut scrollbar_state =
                ScrollbarState::new(self.panel.lines.into()).position(self.panel.scroll.into());

            f.render_stateful_widget(
                scrollbar,
                area.inner(Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut scrollbar_state,
            );
        }
    }
}

/// render file context on top of first line
fn render_file_context(f: &mut ratatui::prelude::Frame<'_>, text: &Text, scroll: usize, area: Rect) {
    if area.height < 1 {
        return;
    }
    /*
    // Find first char of a ratatui Line
    fn first_char_of_line_1(line: &Line) -> Option<char> {
        line.spans().map(|span|
            span.content.chars().next()? // First char of span
        ).next() // Get the first result only
    }
    */
    // Find first char of a ratatui Line
    fn first_char_of_line(line: &Line) -> Option<char> {
        // Spans may have no chars, so we need to try them all
        for span in line.iter() {
            for c in span.content.chars() {
                // Return first char found
                return Some(c);
            }
        }
        None
    }
    // Find the last line that has a letter in first column, before scroll window
    let last_header_line = text
        .iter() // iterate over lines
        .take(scroll+1) // Only lines before and first line in scroll window
        .filter(|&line| // and only lines that start with a letter
            first_char_of_line(line)
            .filter(|&ch| ch.is_alphabetic()) != None)
        .last();
    // If such a line was found, render it as a header on the top row
    if let Some(header_line) = last_header_line {
        let paragraph = Paragraph::new(Text::from(header_line.clone()));
        f.render_widget(paragraph, area);
    }
}

impl DetailsPanel {
    pub fn new() -> Self {
        Self {
            scroll: 0,
            height: 0,
            lines: 0,
            drag_origin: 0.0,
            wrap: true,
        }
    }

    pub fn render_context(&mut self) -> DetailsPanelRenderContext {
        DetailsPanelRenderContext::new(self)
    }

    /// Render the content as a Paragraph
    pub fn render<'a, T>(&mut self, content: T, area: Rect) -> Paragraph<'a>
    where
        T: Into<Text<'a>>,
    {
        let mut paragraph = Paragraph::new(content);

        if self.wrap {
            paragraph = paragraph.wrap(Wrap { trim: false });
        }

        self.height = area.height;
        self.lines = paragraph.line_count(area.width) as u16;

        paragraph = paragraph.scroll((self.scroll.min(self.lines.saturating_sub(1)), 0));

        paragraph
    }

    pub fn scroll_to(&mut self, line_no: u16) {
        self.scroll = line_no.min(self.lines.saturating_sub(1))
    }

    pub fn scroll(&mut self, scroll: isize) {
        self.scroll_to(self.scroll.saturating_add_signed(scroll as i16))
    }

    /// Mark the line where dragging starts. Note that rel_line_no must grow 1
    /// for every scroll line, but it does not matter if scroll=0 is where rel_line_no==0
    ///
    pub fn drag_base(&mut self, rel_line_no: f32) {
        self.drag_origin = rel_line_no * (self.lines as f32) - (self.scroll as f32);
    }

    /// Scroll relative to drag_base.
    pub fn drag_move_to(&mut self, rel_line_no: f32) {
        let drag_target_line = rel_line_no * (self.lines as f32) - self.drag_origin;
        let scroll_target_line = drag_target_line.clamp(0.0, self.lines.into());
        self.scroll_to(scroll_target_line as u16);
    }

    pub fn handle_event(&mut self, details_panel_event: DetailsPanelEvent) {
        match details_panel_event {
            DetailsPanelEvent::ScrollDown => self.scroll(1),
            DetailsPanelEvent::ScrollUp => self.scroll(-1),
            DetailsPanelEvent::ScrollDownHalfPage => self.scroll(self.height as isize / 2),
            DetailsPanelEvent::ScrollUpHalfPage => {
                self.scroll((self.height as isize / 2).saturating_neg())
            }
            DetailsPanelEvent::ScrollDownPage => self.scroll(self.height as isize),
            DetailsPanelEvent::ScrollUpPage => self.scroll((self.height as isize).saturating_neg()),
            DetailsPanelEvent::DragBegin(rel_line) => self.drag_base(rel_line),
            DetailsPanelEvent::DragUpdate(rel_line) => self.drag_move_to(rel_line),
            DetailsPanelEvent::DragEnd(rel_line) => self.drag_move_to(rel_line),
            DetailsPanelEvent::ToggleWrap => self.wrap = !self.wrap,
        }
    }

    /// Handle input. Returns bool of if event was handled
    pub fn input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_event(DetailsPanelEvent::ScrollDown)
            }
            KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_event(DetailsPanelEvent::ScrollUp)
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_event(DetailsPanelEvent::ScrollDownHalfPage)
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_event(DetailsPanelEvent::ScrollUpHalfPage)
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_event(DetailsPanelEvent::ScrollDownPage)
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_event(DetailsPanelEvent::ScrollUpPage)
            }
            KeyCode::Char('W') => self.handle_event(DetailsPanelEvent::ToggleWrap),
            _ => return false,
        };

        true
    }
}
