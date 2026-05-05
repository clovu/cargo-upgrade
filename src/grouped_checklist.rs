use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use std::borrow::Cow;
use std::io::{self, stdout};
use unicode_width::UnicodeWidthStr;

pub(crate) struct ChecklistGroup {
    pub(crate) title: String,
    pub(crate) items: Vec<ChecklistItem>,
}

pub(crate) struct ChecklistItem {
    pub(crate) label: String,
    pub(crate) current: String,
    pub(crate) target: String,
    pub(crate) impact: String,
}

pub(crate) struct ChecklistSelection {
    pub(crate) group_index: usize,
    pub(crate) item_index: usize,
}

pub(crate) fn run_checklist(
    title: &str,
    groups: Vec<ChecklistGroup>,
) -> io::Result<Vec<ChecklistSelection>> {
    Checklist::new(title, groups).run()
}

struct Checklist {
    title: String,
    groups: Vec<Group>,
    cursor: usize,
    scroll: usize,
}

struct Group {
    title: String,
    items: Vec<Item>,
    expanded: bool,
}

struct Item {
    label: String,
    current: String,
    target: String,
    impact: String,
    checked: bool,
}

#[derive(Clone, Copy)]
enum Row {
    Group(usize),
    Item { group: usize, item: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GroupCheckState {
    None,
    Partial,
    All,
}

impl Checklist {
    fn new(title: &str, groups: Vec<ChecklistGroup>) -> Self {
        let groups = groups
            .into_iter()
            .map(|group| Group {
                title: group.title,
                expanded: true,
                items: group
                    .items
                    .into_iter()
                    .map(|item| Item {
                        label: item.label,
                        current: item.current,
                        target: item.target,
                        impact: item.impact,
                        checked: true,
                    })
                    .collect(),
            })
            .collect();

        Self {
            title: title.to_string(),
            groups,
            cursor: 0,
            scroll: 0,
        }
    }

    fn run(mut self) -> io::Result<Vec<ChecklistSelection>> {
        let _session = TerminalSession::enter()?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

        loop {
            terminal.draw(|frame| self.render(frame))?;

            if !event::poll(std::time::Duration::from_millis(16))? {
                continue;
            }

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(Vec::new());
                    }
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(Vec::new()),
                    KeyCode::Up | KeyCode::Char('k') => self.move_up(),
                    KeyCode::Down | KeyCode::Char('j') => self.move_down(),
                    KeyCode::PageUp => self.move_page_up(),
                    KeyCode::PageDown => self.move_page_down(),
                    KeyCode::Home => self.move_home(),
                    KeyCode::End => self.move_end(),
                    KeyCode::Left | KeyCode::Char('h') => self.collapse_or_parent(),
                    KeyCode::Right | KeyCode::Char('l') => self.expand_current_group(),
                    KeyCode::Char(' ') => self.toggle_current_row(),
                    KeyCode::Char('a') => self.toggle_all(),
                    KeyCode::Char('i') => self.invert_selection(),
                    KeyCode::Char('e') => self.toggle_current_group_expansion(),
                    KeyCode::Enter => return Ok(self.into_selection()),
                    _ => {}
                }
            }
        }
    }

    fn render(&mut self, frame: &mut Frame<'_>) {
        let area = frame.area();
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

        self.render_summary(frame, chunks[0]);
        self.render_body(frame, chunks[1]);
        self.render_footer(frame, chunks[2]);
    }

    fn render_summary(&self, frame: &mut Frame<'_>, area: Rect) {
        let selected = self.selected_items();
        let total = self.total_items();
        let cursor = self.cursor_summary();
        let lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("{selected}/{total} selected"),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    section_summary(&self.groups),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Line::from(vec![
                Span::styled("cursor ", Style::default().fg(Color::DarkGray)),
                Span::raw(fit(&cursor, area.width as usize).into_owned()),
            ]),
        ];

        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_body(&mut self, frame: &mut Frame<'_>, area: Rect) {
        if area.width >= 76 {
            let chunks =
                Layout::horizontal([Constraint::Percentage(62), Constraint::Percentage(38)])
                    .split(area);

            self.render_tree(frame, chunks[0]);
            self.render_details(frame, chunks[1]);
        } else {
            self.render_tree(frame, area);
        }
    }

    fn render_tree(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let block = Block::default().title(" Tree ").borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let rows = self.visible_rows();
        let visible_rows = inner.height as usize;
        self.keep_cursor_visible(visible_rows);

        let lines = rows
            .iter()
            .enumerate()
            .skip(self.scroll)
            .take(visible_rows)
            .map(|(index, row)| self.render_tree_row(*row, index == self.cursor, inner.width))
            .collect::<Vec<_>>();

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_details(&self, frame: &mut Frame<'_>, area: Rect) {
        let block = Block::default().title(" Details ").borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = match self.current_row() {
            Some(Row::Group(group_index)) => self.group_detail_lines(group_index, inner.width),
            Some(Row::Item { group, item }) => self.item_detail_lines(group, item, inner.width),
            None => vec![Line::from("No updates")],
        };

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_footer(&self, frame: &mut Frame<'_>, area: Rect) {
        let text =
            "Space select  Left/Right fold  e fold  a all/none  i invert  Enter apply  Esc cancel";
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                fit(text, area.width as usize).into_owned(),
                Style::default().fg(Color::DarkGray),
            ))),
            area,
        );
    }

    fn render_tree_row(&self, row: Row, active: bool, width: u16) -> Line<'static> {
        match row {
            Row::Group(group_index) => self.render_group_row(group_index, active, width),
            Row::Item { group, item } => self.render_item_row(group, item, active, width),
        }
    }

    fn render_group_row(&self, group_index: usize, active: bool, width: u16) -> Line<'static> {
        let group = &self.groups[group_index];
        let marker = group_marker(group_check_state(&group.items));
        let expander = if group.expanded { "-" } else { "+" };
        let selected = group.items.iter().filter(|item| item.checked).count();
        let text = format!(
            "{marker} {expander} {} ({selected}/{})",
            group.title,
            group.items.len()
        );
        let style = active_style(
            active,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

        Line::from(vec![
            cursor_span(active),
            Span::styled(fit(&text, row_text_width(width)).into_owned(), style),
        ])
    }

    fn render_item_row(
        &self,
        group_index: usize,
        item_index: usize,
        active: bool,
        width: u16,
    ) -> Line<'static> {
        let group = &self.groups[group_index];
        let item = &group.items[item_index];
        let branch = if item_index + 1 == group.items.len() {
            "`-"
        } else {
            "|-"
        };
        let marker = if item.checked { "[x]" } else { "[ ]" };
        let prefix = format!("  {branch} {marker} ");
        let summary = format!(
            "{}{}  {} -> {}  ",
            prefix, item.label, item.current, item.target
        );
        let remaining =
            row_text_width(width).saturating_sub(UnicodeWidthStr::width(summary.as_str()));
        let style = active_style(active, Style::default().fg(Color::Gray));
        let impact_style = active_style(active, impact_style(&item.impact));

        if remaining > item.impact.len() {
            return Line::from(vec![
                cursor_span(active),
                Span::styled(summary, style),
                Span::styled(item.impact.clone(), impact_style),
            ]);
        }

        let text = format!(
            "{}{}  {} -> {}  {}",
            prefix, item.label, item.current, item.target, item.impact
        );

        Line::from(vec![
            cursor_span(active),
            Span::styled(fit(&text, row_text_width(width)).into_owned(), style),
        ])
    }

    fn group_detail_lines(&self, group_index: usize, width: u16) -> Vec<Line<'static>> {
        let group = &self.groups[group_index];
        let selected = group.items.iter().filter(|item| item.checked).count();
        let state = match group_check_state(&group.items) {
            GroupCheckState::None => "none selected",
            GroupCheckState::Partial => "partially selected",
            GroupCheckState::All => "all selected",
        };
        let expanded = if group.expanded {
            "expanded"
        } else {
            "collapsed"
        };

        vec![
            detail_line("Section", &group.title, width),
            detail_line("Packages", &group.items.len().to_string(), width),
            detail_line(
                "Selected",
                &format!("{selected}/{}", group.items.len()),
                width,
            ),
            detail_line("State", state, width),
            detail_line("Tree", expanded, width),
            Line::raw(""),
            Line::styled(
                fit("Space toggles the whole section.", width as usize).into_owned(),
                Style::default().fg(Color::DarkGray),
            ),
            Line::styled(
                fit("Left/Right collapses or expands children.", width as usize).into_owned(),
                Style::default().fg(Color::DarkGray),
            ),
        ]
    }

    fn item_detail_lines(
        &self,
        group_index: usize,
        item_index: usize,
        width: u16,
    ) -> Vec<Line<'static>> {
        let group = &self.groups[group_index];
        let item = &group.items[item_index];
        let selected = if item.checked { "yes" } else { "no" };

        vec![
            detail_line("Package", &item.label, width),
            detail_line("Section", &group.title, width),
            detail_line("Current", &item.current, width),
            detail_line("Target", &item.target, width),
            detail_line("Impact", &item.impact, width),
            detail_line("Selected", selected, width),
            Line::raw(""),
            Line::styled(
                fit(
                    "Review impact first; deselect risky upgrades.",
                    width as usize,
                )
                .into_owned(),
                Style::default().fg(Color::DarkGray),
            ),
        ]
    }

    fn visible_rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();

        for (group_index, group) in self.groups.iter().enumerate() {
            rows.push(Row::Group(group_index));

            if group.expanded {
                for (item_index, _) in group.items.iter().enumerate() {
                    rows.push(Row::Item {
                        group: group_index,
                        item: item_index,
                    });
                }
            }
        }

        rows
    }

    fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn move_down(&mut self) {
        if self.cursor < self.last_visible_row_index() {
            self.cursor += 1;
        }
    }

    fn move_page_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(10);
    }

    fn move_page_down(&mut self) {
        self.cursor = self
            .cursor
            .saturating_add(10)
            .min(self.last_visible_row_index());
    }

    fn move_home(&mut self) {
        self.cursor = 0;
    }

    fn move_end(&mut self) {
        self.cursor = self.last_visible_row_index();
    }

    fn collapse_or_parent(&mut self) {
        match self.current_row() {
            Some(Row::Group(group_index)) => {
                self.groups[group_index].expanded = false;
                self.clamp_cursor();
            }
            Some(Row::Item { group, .. }) => {
                if let Some(parent) = self
                    .visible_rows()
                    .iter()
                    .position(|row| matches!(row, Row::Group(group_index) if *group_index == group))
                {
                    self.cursor = parent;
                }
            }
            None => {}
        }
    }

    fn expand_current_group(&mut self) {
        if let Some(Row::Group(group_index)) = self.current_row() {
            self.groups[group_index].expanded = true;
        }
    }

    fn toggle_current_group_expansion(&mut self) {
        if let Some(Row::Group(group_index)) = self.current_row() {
            self.groups[group_index].expanded = !self.groups[group_index].expanded;
            self.clamp_cursor();
        }
    }

    fn toggle_current_row(&mut self) {
        match self.current_row() {
            Some(Row::Group(group_index)) => self.toggle_group(group_index),
            Some(Row::Item { group, item }) => self.toggle_item(group, item),
            None => {}
        }
    }

    fn toggle_group(&mut self, group_index: usize) {
        let group = &mut self.groups[group_index];
        let should_check = group_check_state(&group.items) != GroupCheckState::All;

        for item in &mut group.items {
            item.checked = should_check;
        }
    }

    fn toggle_item(&mut self, group_index: usize, item_index: usize) {
        let item = &mut self.groups[group_index].items[item_index];
        item.checked = !item.checked;
    }

    fn toggle_all(&mut self) {
        let should_check = self.selected_items() != self.total_items();

        for item in self.groups.iter_mut().flat_map(|group| &mut group.items) {
            item.checked = should_check;
        }
    }

    fn invert_selection(&mut self) {
        for item in self.groups.iter_mut().flat_map(|group| &mut group.items) {
            item.checked = !item.checked;
        }
    }

    fn keep_cursor_visible(&mut self, visible_rows: usize) {
        if visible_rows == 0 {
            self.scroll = self.cursor;
            return;
        }

        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        }

        if self.cursor >= self.scroll.saturating_add(visible_rows) {
            self.scroll = self.cursor.saturating_sub(visible_rows - 1);
        }
    }

    fn current_row(&self) -> Option<Row> {
        self.visible_rows().get(self.cursor).copied()
    }

    fn cursor_summary(&self) -> String {
        match self.current_row() {
            Some(Row::Group(group_index)) => {
                let group = &self.groups[group_index];
                let selected = group.items.iter().filter(|item| item.checked).count();
                format!(
                    "{} section  {selected}/{} selected",
                    group.title,
                    group.items.len()
                )
            }
            Some(Row::Item { group, item }) => {
                let item = &self.groups[group].items[item];
                format!(
                    "{}  {} -> {}  {}",
                    item.label, item.current, item.target, item.impact
                )
            }
            None => "No package under cursor".to_string(),
        }
    }

    fn selected_items(&self) -> usize {
        self.groups
            .iter()
            .flat_map(|group| &group.items)
            .filter(|item| item.checked)
            .count()
    }

    fn total_items(&self) -> usize {
        self.groups.iter().map(|group| group.items.len()).sum()
    }

    fn last_visible_row_index(&self) -> usize {
        self.visible_rows().len().saturating_sub(1)
    }

    fn clamp_cursor(&mut self) {
        self.cursor = self.cursor.min(self.last_visible_row_index());
    }

    fn into_selection(self) -> Vec<ChecklistSelection> {
        self.groups
            .into_iter()
            .enumerate()
            .flat_map(|(group_index, group)| {
                group
                    .items
                    .into_iter()
                    .enumerate()
                    .filter(|(_, item)| item.checked)
                    .map(move |(item_index, _)| ChecklistSelection {
                        group_index,
                        item_index,
                    })
            })
            .collect()
    }
}

struct TerminalSession;

impl TerminalSession {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
    }
}

fn active_style(active: bool, style: Style) -> Style {
    if active {
        style.fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        style
    }
}

fn cursor_span(active: bool) -> Span<'static> {
    if active {
        Span::styled("> ", Style::default().fg(Color::Yellow))
    } else {
        Span::raw("  ")
    }
}

fn detail_line(label: &str, value: &str, width: u16) -> Line<'static> {
    let text = format!("{label:<9} {value}");
    Line::raw(fit(&text, width as usize).into_owned())
}

fn group_marker(state: GroupCheckState) -> &'static str {
    match state {
        GroupCheckState::None => "[ ]",
        GroupCheckState::Partial => "[-]",
        GroupCheckState::All => "[x]",
    }
}

fn impact_style(impact: &str) -> Style {
    match impact {
        "major" => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        "minor" => Style::default().fg(Color::Yellow),
        "patch" => Style::default().fg(Color::Green),
        _ => Style::default().fg(Color::Blue),
    }
}

fn group_check_state(items: &[Item]) -> GroupCheckState {
    let checked = items.iter().filter(|item| item.checked).count();

    match (checked, items.len()) {
        (0, _) => GroupCheckState::None,
        (count, total) if count == total => GroupCheckState::All,
        _ => GroupCheckState::Partial,
    }
}

fn section_summary(groups: &[Group]) -> String {
    groups
        .iter()
        .map(|group| {
            let selected = group.items.iter().filter(|item| item.checked).count();
            format!("{} {selected}/{}", group.title, group.items.len())
        })
        .collect::<Vec<_>>()
        .join("  ")
}

fn row_text_width(width: u16) -> usize {
    usize::from(width).saturating_sub(2)
}

fn fit(text: &str, width: usize) -> Cow<'_, str> {
    if UnicodeWidthStr::width(text) <= width {
        return Cow::Borrowed(text);
    }

    if width <= 3 {
        return Cow::Owned(".".repeat(width));
    }

    let mut output = String::new();
    let target = width - 3;

    for character in text.chars() {
        let mut next = output.clone();
        next.push(character);

        if UnicodeWidthStr::width(next.as_str()) > target {
            break;
        }

        output.push(character);
    }

    output.push_str("...");
    Cow::Owned(output)
}

#[cfg(test)]
mod tests {
    use super::{Group, GroupCheckState, Item, Row, fit, group_check_state};

    fn item(checked: bool) -> Item {
        Item {
            label: String::new(),
            current: String::new(),
            target: String::new(),
            impact: String::new(),
            checked,
        }
    }

    #[test]
    fn derives_none_when_nothing_is_checked() {
        let items = vec![item(false), item(false)];
        assert_eq!(group_check_state(&items), GroupCheckState::None);
    }

    #[test]
    fn derives_partial_when_only_some_items_are_checked() {
        let items = vec![item(true), item(false)];
        assert_eq!(group_check_state(&items), GroupCheckState::Partial);
    }

    #[test]
    fn derives_all_when_everything_is_checked() {
        let items = vec![item(true), item(true)];
        assert_eq!(group_check_state(&items), GroupCheckState::All);
    }

    #[test]
    fn derives_none_for_empty_group() {
        assert_eq!(group_check_state(&[]), GroupCheckState::None);
    }

    #[test]
    fn truncates_long_text() {
        assert_eq!(fit("serde_json", 8), "serde...");
    }

    #[test]
    fn visible_rows_respect_group_expansion() {
        let checklist = super::Checklist {
            title: String::new(),
            cursor: 0,
            scroll: 0,
            groups: vec![
                Group {
                    title: "dependencies".into(),
                    expanded: true,
                    items: vec![item(true), item(true)],
                },
                Group {
                    title: "devDependencies".into(),
                    expanded: false,
                    items: vec![item(true)],
                },
            ],
        };

        let rows = checklist.visible_rows();

        assert_eq!(rows.len(), 4);
        assert!(matches!(rows[0], Row::Group(0)));
        assert!(matches!(rows[1], Row::Item { group: 0, item: 0 }));
        assert!(matches!(rows[2], Row::Item { group: 0, item: 1 }));
        assert!(matches!(rows[3], Row::Group(1)));
    }

    #[test]
    fn toggling_group_checks_every_item_when_not_all_are_checked() {
        let mut group = Group {
            title: "dependencies".into(),
            expanded: true,
            items: vec![item(false), item(true)],
        };

        let should_check = group_check_state(&group.items) != GroupCheckState::All;
        for item in &mut group.items {
            item.checked = should_check;
        }

        assert!(group.items.iter().all(|item| item.checked));
    }
}
