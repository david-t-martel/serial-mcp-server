//! UI rendering for the TUI.

use super::app::{App, DataLine, FocusArea, Mode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io::{self, Stdout};

/// Set up the terminal for TUI rendering.
pub fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

/// Restore the terminal to normal mode.
pub fn restore_terminal(mut terminal: Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Render the entire UI.
pub fn render(app: &App, frame: &mut Frame) {
    let size = frame.area();

    // Main layout: header, body, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(10),   // Body
            Constraint::Length(3), // Input
            Constraint::Length(1), // Status bar
        ])
        .split(size);

    render_header(app, frame, chunks[0]);
    render_body(app, frame, chunks[1]);
    render_input(app, frame, chunks[2]);
    render_status_bar(app, frame, chunks[3]);

    // Overlay modals
    match app.mode {
        Mode::Help => render_help_overlay(app, frame, size),
        Mode::ConfigEdit => render_config_overlay(app, frame, size),
        _ => {}
    }
}

fn render_header(app: &App, frame: &mut Frame, area: Rect) {
    let port_info = match &app.connected_port {
        Some(port) => format!("{} @ {}", port, app.config.serial.default_baud),
        None => "Not connected".to_string(),
    };

    let status = if app.connected_port.is_some() {
        "Connected"
    } else {
        "Disconnected"
    };

    let header = Line::from(vec![
        Span::styled(
            " rust-comm TUI ",
            Style::default()
                .fg(Color::from(app.theme.accent))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(port_info, Style::default().fg(Color::from(app.theme.fg))),
        Span::raw(" | "),
        Span::styled(
            status,
            Style::default().fg(if app.connected_port.is_some() {
                Color::from(app.theme.success_color)
            } else {
                Color::from(app.theme.inactive)
            }),
        ),
        Span::raw(" | "),
        Span::styled(
            app.uptime_string(),
            Style::default().fg(Color::from(app.theme.fg)),
        ),
    ]);

    let header_widget =
        Paragraph::new(header).style(Style::default().bg(Color::from(app.theme.selection)));

    frame.render_widget(header_widget, area);
}

fn render_body(app: &App, frame: &mut Frame, area: Rect) {
    // Split into port list and terminal
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(20), Constraint::Min(40)])
        .split(area);

    render_port_list(app, frame, chunks[0]);
    render_terminal(app, frame, chunks[1]);
}

fn render_port_list(app: &App, frame: &mut Frame, area: Rect) {
    let is_focused = app.focus == FocusArea::PortList;

    let items: Vec<ListItem> = app
        .available_ports
        .iter()
        .enumerate()
        .map(|(i, port)| {
            let style = if i == app.selected_port {
                Style::default()
                    .fg(Color::from(app.theme.fg))
                    .bg(Color::from(app.theme.selection))
                    .add_modifier(Modifier::BOLD)
            } else if app.connected_port.as_ref() == Some(port) {
                Style::default().fg(Color::from(app.theme.success_color))
            } else {
                Style::default().fg(Color::from(app.theme.fg))
            };

            let prefix = if app.connected_port.as_ref() == Some(port) {
                "● "
            } else if i == app.selected_port {
                "> "
            } else {
                "  "
            };

            ListItem::new(format!("{}{}", prefix, port)).style(style)
        })
        .collect();

    let border_style = if is_focused {
        Style::default().fg(Color::from(app.theme.accent))
    } else {
        Style::default().fg(Color::from(app.theme.border))
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Ports "),
    );

    frame.render_widget(list, area);
}

fn render_terminal(app: &App, frame: &mut Frame, area: Rect) {
    let is_focused = app.focus == FocusArea::Terminal;

    let border_style = if is_focused {
        Style::default().fg(Color::from(app.theme.accent))
    } else {
        Style::default().fg(Color::from(app.theme.border))
    };

    let title = if app.show_hex {
        " Terminal (Hex) "
    } else {
        " Terminal "
    };

    let lines: Vec<Line> = app
        .rx_buffer
        .iter()
        .skip(app.scroll_offset)
        .take(area.height.saturating_sub(2) as usize)
        .map(|data_line| format_data_line(app, data_line))
        .collect();

    let terminal_widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(terminal_widget, area);
}

fn format_data_line<'a>(app: &App, line: &DataLine) -> Line<'a> {
    let direction_span = if line.is_tx {
        Span::styled("TX: ", Style::default().fg(Color::from(app.theme.tx_color)))
    } else {
        Span::styled("RX: ", Style::default().fg(Color::from(app.theme.rx_color)))
    };

    let data_str = if app.show_hex {
        line.data
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        String::from_utf8_lossy(&line.data)
            .replace('\r', "")
            .replace('\n', "↵")
    };

    let data_span = Span::styled(
        data_str,
        Style::default().fg(if line.is_tx {
            Color::from(app.theme.tx_color)
        } else {
            Color::from(app.theme.rx_color)
        }),
    );

    Line::from(vec![direction_span, data_span])
}

fn render_input(app: &App, frame: &mut Frame, area: Rect) {
    let is_focused = app.focus == FocusArea::Input || app.mode == Mode::Insert;

    let border_style = if is_focused {
        Style::default().fg(Color::from(app.theme.accent))
    } else {
        Style::default().fg(Color::from(app.theme.border))
    };

    let mode_indicator = match app.mode {
        Mode::Normal => "[NORMAL]",
        Mode::Insert => "[INSERT]",
        Mode::Command => "[COMMAND]",
        _ => "",
    };

    let title = format!(" Input {} ", mode_indicator);

    let input_content = if app.mode == Mode::Command {
        format!(":{}", app.input)
    } else {
        app.input.clone()
    };

    let input_widget = Paragraph::new(input_content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title),
    );

    frame.render_widget(input_widget, area);

    // Set cursor position
    if app.mode == Mode::Insert || app.mode == Mode::Command {
        let cursor_x = area.x + 1 + app.cursor_pos as u16;
        if app.mode == Mode::Command {
            frame.set_cursor_position((cursor_x + 1, area.y + 1));
        } else {
            frame.set_cursor_position((cursor_x, area.y + 1));
        }
    }
}

fn render_status_bar(app: &App, frame: &mut Frame, area: Rect) {
    let status_text = app
        .status_message
        .clone()
        .unwrap_or_else(|| "Ready".to_string());

    let keybinds = "q:quit  i:insert  Tab:focus  Ctrl+H:hex  F1:help  :cmd";

    let status = Line::from(vec![
        Span::styled(
            format!(" {} ", status_text),
            Style::default().fg(Color::from(app.theme.fg)),
        ),
        Span::raw(" | "),
        Span::styled(
            keybinds,
            Style::default().fg(Color::from(app.theme.inactive)),
        ),
    ]);

    let status_widget =
        Paragraph::new(status).style(Style::default().bg(Color::from(app.theme.selection)));

    frame.render_widget(status_widget, area);
}

fn render_help_overlay(app: &App, frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 70, area);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(Span::styled(
            "Keybindings",
            Style::default()
                .fg(Color::from(app.theme.accent))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Normal Mode:"),
        Line::from("  q          - Quit"),
        Line::from("  i          - Enter insert mode"),
        Line::from("  :          - Enter command mode"),
        Line::from("  Tab        - Cycle focus"),
        Line::from("  Ctrl+H     - Toggle hex view"),
        Line::from("  Ctrl+L     - Clear terminal"),
        Line::from("  F1 / ?     - Show help"),
        Line::from("  j/k        - Move selection"),
        Line::from("  Enter      - Connect/send"),
        Line::from(""),
        Line::from("Insert Mode:"),
        Line::from("  Esc        - Return to normal"),
        Line::from("  Enter      - Send data"),
        Line::from("  Tab        - Autocomplete"),
        Line::from("  Up/Down    - History navigation"),
        Line::from(""),
        Line::from("Commands (:)"),
        Line::from("  :quit      - Exit application"),
        Line::from("  :config    - Open config editor"),
        Line::from("  :hex       - Toggle hex view"),
        Line::from("  :clear     - Clear terminal"),
        Line::from("  :refresh   - Refresh port list"),
        Line::from(""),
        Line::from(Span::styled(
            "Press Esc or F1 to close",
            Style::default().fg(Color::from(app.theme.inactive)),
        )),
    ];

    let help_widget = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::from(app.theme.accent)))
                .title(" Help ")
                .style(Style::default().bg(Color::from(app.theme.bg))),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(help_widget, popup_area);
}

fn render_config_overlay(app: &App, frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(70, 80, area);

    frame.render_widget(Clear, popup_area);

    let config_text = vec![
        Line::from(Span::styled(
            "Configuration",
            Style::default()
                .fg(Color::from(app.theme.accent))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("Theme: {}", app.config.tui.theme)),
        Line::from(format!("Default Baud: {}", app.config.serial.default_baud)),
        Line::from(format!(
            "Timeout: {} ms",
            app.config.serial.default_timeout_ms
        )),
        Line::from(format!("History Size: {}", app.config.tui.history_size)),
        Line::from(""),
        Line::from(Span::styled(
            "Press Esc to close",
            Style::default().fg(Color::from(app.theme.inactive)),
        )),
    ];

    let config_widget = Paragraph::new(config_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::from(app.theme.accent)))
                .title(" Config ")
                .style(Style::default().bg(Color::from(app.theme.bg))),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(config_widget, popup_area);
}

/// Create a centered rectangle with the given percentage of the parent area.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
