//! TUI Application state and main loop.

use crate::config::{Config, ConfigLoader};
use crate::service::PortService;
use crate::AppState as CoreAppState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::VecDeque;
use std::io;
// use std::sync::{Arc, Mutex}; // TODO: Will be needed for shared state
use std::time::Instant;

use super::event::{Event, EventHandler};
use super::theme::Theme;
use super::ui;

/// Application mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Normal mode - navigation and viewing
    Normal,
    /// Insert mode - typing commands
    Insert,
    /// Command mode - vim-like : commands
    Command,
    /// Config editor mode
    ConfigEdit,
    /// Hex view mode
    HexView,
    /// Help overlay
    Help,
    /// Script editor mode
    #[cfg(feature = "scripting")]
    ScriptEdit,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Normal
    }
}

/// Focus area in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusArea {
    /// Port list panel
    PortList,
    /// Terminal output
    Terminal,
    /// Input field
    Input,
    /// Config editor
    ConfigEditor,
}

impl Default for FocusArea {
    fn default() -> Self {
        Self::Input
    }
}

/// A line of data in the terminal buffer.
#[derive(Debug, Clone)]
pub struct DataLine {
    /// Timestamp when data was received/sent
    pub timestamp: Instant,
    /// Whether this is TX (true) or RX (false)
    pub is_tx: bool,
    /// The data bytes
    pub data: Vec<u8>,
}

/// Application state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    /// Application is running
    Running,
    /// Application should quit
    Quitting,
}

/// Main TUI application.
pub struct App {
    /// Current app state
    pub state: AppState,
    /// Current mode
    pub mode: Mode,
    /// Current theme
    pub theme: Theme,
    /// Focus area
    pub focus: FocusArea,

    /// Configuration
    pub config: Config,

    /// Port service for serial operations
    pub port_service: Option<PortService>,

    /// Receive buffer for terminal display
    pub rx_buffer: VecDeque<DataLine>,
    /// Maximum buffer size
    pub buffer_size: usize,

    /// Current input text
    pub input: String,
    /// Cursor position in input
    pub cursor_pos: usize,

    /// Command history
    pub history: Vec<String>,
    /// Current history index (for up/down navigation)
    pub history_index: Option<usize>,

    /// Available ports (for port list)
    pub available_ports: Vec<String>,
    /// Selected port index
    pub selected_port: usize,

    /// Currently connected port name
    pub connected_port: Option<String>,

    /// Connection start time (for uptime display)
    pub connect_time: Option<Instant>,

    /// Status message to display
    pub status_message: Option<String>,

    /// Show hex view
    pub show_hex: bool,

    /// Scroll offset for terminal
    pub scroll_offset: usize,
}

impl App {
    /// Create a new application instance.
    pub fn new() -> io::Result<Self> {
        let config = ConfigLoader::with_defaults().into_config();
        let theme = Theme::by_name(&config.tui.theme)
            .cloned()
            .unwrap_or_default();

        Ok(Self {
            state: AppState::Running,
            mode: Mode::Normal,
            theme,
            focus: FocusArea::Input,
            config,
            port_service: None,
            rx_buffer: VecDeque::with_capacity(1000),
            buffer_size: 1000,
            input: String::new(),
            cursor_pos: 0,
            history: Vec::new(),
            history_index: None,
            available_ports: Vec::new(),
            selected_port: 0,
            connected_port: None,
            connect_time: None,
            status_message: None,
            show_hex: false,
            scroll_offset: 0,
        })
    }

    /// Create app with existing port state.
    pub fn with_port_state(port_state: CoreAppState) -> io::Result<Self> {
        let mut app = Self::new()?;
        app.port_service = Some(PortService::new(port_state));
        Ok(app)
    }

    /// Run the application main loop.
    pub async fn run(&mut self) -> io::Result<()> {
        // Set up terminal
        let mut terminal = ui::setup_terminal()?;

        // Create event handler
        let tick_rate = self.config.tui.refresh_interval();
        let events = EventHandler::new(tick_rate);

        // Discover available ports
        self.refresh_ports();

        // Main loop
        while self.state == AppState::Running {
            // Draw UI
            terminal.draw(|frame| ui::render(self, frame))?;

            // Handle events
            match events.next() {
                Ok(Event::Tick) => {
                    // Periodic updates (e.g., refresh port list)
                }
                Ok(Event::Key(key)) => {
                    self.handle_key(key);
                }
                Ok(Event::Mouse(_mouse)) => {
                    // Mouse handling (optional)
                }
                Ok(Event::Resize(_, _)) => {
                    // Terminal will auto-resize
                }
                Ok(Event::SerialRx(data)) => {
                    self.add_rx_data(data);
                }
                Ok(Event::PortConnected(port)) => {
                    self.connected_port = Some(port.clone());
                    self.connect_time = Some(Instant::now());
                    self.status_message = Some(format!("Connected to {}", port));
                }
                Ok(Event::PortDisconnected(port)) => {
                    if self.connected_port.as_ref() == Some(&port) {
                        self.connected_port = None;
                        self.connect_time = None;
                    }
                    self.status_message = Some(format!("Disconnected from {}", port));
                }
                Ok(Event::Error(err)) => {
                    self.status_message = Some(format!("Error: {}", err));
                }
                Err(_) => {
                    self.state = AppState::Quitting;
                }
            }
        }

        // Restore terminal
        ui::restore_terminal(terminal)?;

        Ok(())
    }

    /// Handle keyboard input.
    fn handle_key(&mut self, key: KeyEvent) {
        match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::Insert => self.handle_insert_key(key),
            Mode::Command => self.handle_command_key(key),
            Mode::Help => self.handle_help_key(key),
            Mode::HexView => self.handle_hex_key(key),
            Mode::ConfigEdit => self.handle_config_key(key),
            #[cfg(feature = "scripting")]
            Mode::ScriptEdit => self.handle_script_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.state = AppState::Quitting,
            KeyCode::Char('i') => self.mode = Mode::Insert,
            KeyCode::Char(':') => {
                self.mode = Mode::Command;
                self.input.clear();
                self.cursor_pos = 0;
            }
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.show_hex = !self.show_hex;
            }
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.rx_buffer.clear();
            }
            KeyCode::F(1) | KeyCode::Char('?') => self.mode = Mode::Help,
            KeyCode::Tab => self.cycle_focus(),
            KeyCode::Up => self.scroll_up(),
            KeyCode::Down => self.scroll_down(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::PageDown => self.page_down(),
            KeyCode::Enter => {
                if self.focus == FocusArea::PortList && !self.available_ports.is_empty() {
                    self.connect_selected_port();
                } else {
                    self.mode = Mode::Insert;
                }
            }
            KeyCode::Char('j') => self.move_selection_down(),
            KeyCode::Char('k') => self.move_selection_up(),
            _ => {}
        }
    }

    fn handle_insert_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Enter => {
                self.send_input();
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.input.remove(self.cursor_pos);
                }
            }
            KeyCode::Delete => {
                if self.cursor_pos < self.input.len() {
                    self.input.remove(self.cursor_pos);
                }
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_pos < self.input.len() {
                    self.cursor_pos += 1;
                }
            }
            KeyCode::Home => self.cursor_pos = 0,
            KeyCode::End => self.cursor_pos = self.input.len(),
            KeyCode::Up => self.history_previous(),
            KeyCode::Down => self.history_next(),
            KeyCode::Tab => self.autocomplete(),
            KeyCode::Char(c) => {
                self.input.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
            }
            _ => {}
        }
    }

    fn handle_command_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.input.clear();
            }
            KeyCode::Enter => {
                self.execute_command();
                self.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.input.remove(self.cursor_pos);
                }
                if self.input.is_empty() {
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Char(c) => {
                self.input.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
            }
            _ => {}
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::F(1) | KeyCode::Char('q') => {
                self.mode = Mode::Normal;
            }
            _ => {}
        }
    }

    fn handle_hex_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.show_hex = false;
                self.mode = Mode::Normal;
            }
            KeyCode::Up => self.scroll_up(),
            KeyCode::Down => self.scroll_down(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::PageDown => self.page_down(),
            _ => {}
        }
    }

    fn handle_config_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            _ => {}
        }
    }

    #[cfg(feature = "scripting")]
    fn handle_script_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            _ => {}
        }
    }

    /// Cycle through focus areas.
    fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            FocusArea::PortList => FocusArea::Terminal,
            FocusArea::Terminal => FocusArea::Input,
            FocusArea::Input => FocusArea::PortList,
            FocusArea::ConfigEditor => FocusArea::PortList,
        };
    }

    /// Move selection up in port list.
    fn move_selection_up(&mut self) {
        if self.selected_port > 0 {
            self.selected_port -= 1;
        }
    }

    /// Move selection down in port list.
    fn move_selection_down(&mut self) {
        if self.selected_port < self.available_ports.len().saturating_sub(1) {
            self.selected_port += 1;
        }
    }

    /// Scroll terminal up.
    fn scroll_up(&mut self) {
        if self.scroll_offset < self.rx_buffer.len().saturating_sub(1) {
            self.scroll_offset += 1;
        }
    }

    /// Scroll terminal down.
    fn scroll_down(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    /// Page up in terminal.
    fn page_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(10);
        let max = self.rx_buffer.len().saturating_sub(1);
        if self.scroll_offset > max {
            self.scroll_offset = max;
        }
    }

    /// Page down in terminal.
    fn page_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(10);
    }

    /// Navigate to previous history entry.
    fn history_previous(&mut self) {
        if self.history.is_empty() {
            return;
        }

        let new_index = match self.history_index {
            None => Some(self.history.len() - 1),
            Some(0) => Some(0),
            Some(i) => Some(i - 1),
        };

        if let Some(idx) = new_index {
            self.input = self.history[idx].clone();
            self.cursor_pos = self.input.len();
            self.history_index = new_index;
        }
    }

    /// Navigate to next history entry.
    fn history_next(&mut self) {
        if let Some(idx) = self.history_index {
            if idx >= self.history.len() - 1 {
                self.input.clear();
                self.cursor_pos = 0;
                self.history_index = None;
            } else {
                self.history_index = Some(idx + 1);
                self.input = self.history[idx + 1].clone();
                self.cursor_pos = self.input.len();
            }
        }
    }

    /// Autocomplete current input.
    fn autocomplete(&mut self) {
        // TODO: Implement fuzzy completion
        self.status_message = Some("Autocomplete not yet implemented".to_string());
    }

    /// Send the current input.
    fn send_input(&mut self) {
        if self.input.is_empty() {
            return;
        }

        let data = self.input.clone();

        // Add to history
        if self.history.last() != Some(&data) {
            self.history.push(data.clone());
            if self.history.len() > self.config.tui.history_size {
                self.history.remove(0);
            }
        }
        self.history_index = None;

        // Add TX data to buffer
        let mut tx_data = data.as_bytes().to_vec();
        tx_data.extend_from_slice(b"\r\n");

        self.rx_buffer.push_back(DataLine {
            timestamp: Instant::now(),
            is_tx: true,
            data: tx_data.clone(),
        });

        // Trim buffer if needed
        while self.rx_buffer.len() > self.buffer_size {
            self.rx_buffer.pop_front();
        }

        // Clear input
        self.input.clear();
        self.cursor_pos = 0;

        // TODO: Actually send to serial port
        self.status_message = Some(format!("Sent: {}", data));
    }

    /// Execute a command mode command.
    fn execute_command(&mut self) {
        let cmd = self.input.trim().to_lowercase();
        match cmd.as_str() {
            "q" | "quit" => self.state = AppState::Quitting,
            "config" => self.mode = Mode::ConfigEdit,
            "hex" => self.show_hex = !self.show_hex,
            "clear" => self.rx_buffer.clear(),
            "help" => self.mode = Mode::Help,
            "refresh" => self.refresh_ports(),
            _ => {
                self.status_message = Some(format!("Unknown command: {}", cmd));
            }
        }
        self.input.clear();
    }

    /// Refresh the list of available ports.
    fn refresh_ports(&mut self) {
        match serialport::available_ports() {
            Ok(ports) => {
                self.available_ports = ports.into_iter().map(|p| p.port_name).collect();
                if self.selected_port >= self.available_ports.len() {
                    self.selected_port = self.available_ports.len().saturating_sub(1);
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to list ports: {}", e));
            }
        }
    }

    /// Connect to the selected port.
    fn connect_selected_port(&mut self) {
        if self.available_ports.is_empty() {
            self.status_message = Some("No ports available".to_string());
            return;
        }

        let port_name = self.available_ports[self.selected_port].clone();
        self.status_message = Some(format!("Connecting to {}...", port_name));

        // TODO: Actually connect using PortService
        self.connected_port = Some(port_name.clone());
        self.connect_time = Some(Instant::now());
    }

    /// Add received data to the buffer.
    pub fn add_rx_data(&mut self, data: Vec<u8>) {
        self.rx_buffer.push_back(DataLine {
            timestamp: Instant::now(),
            is_tx: false,
            data,
        });

        while self.rx_buffer.len() > self.buffer_size {
            self.rx_buffer.pop_front();
        }
    }

    /// Get uptime string.
    pub fn uptime_string(&self) -> String {
        match self.connect_time {
            Some(start) => {
                let elapsed = start.elapsed();
                let secs = elapsed.as_secs();
                let hours = secs / 3600;
                let mins = (secs % 3600) / 60;
                let secs = secs % 60;
                format!("{:02}:{:02}:{:02}", hours, mins, secs)
            }
            None => "--:--:--".to_string(),
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new().expect("Failed to create default app")
    }
}
