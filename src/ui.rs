//! Terra Store v1.0 - Terminal User Interface
//!
//! Split-pane TUI with instant search powered by Arena-based indexing.
//! Includes History, Audit (with TerraFlow feature), and Universal (Flatpak) modes.

use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::database::PackageDatabase;
use crate::flatpak::FlatpakDatabase;
use crate::history::History;
use crate::package::PackageSource;
use crate::repos::RepoManager;
#[cfg(feature = "terraflow")]
use crate::terraflow::{AuditResult, TerraFlow};
use crate::theme::Theme;

/// Maximum results to display
const MAX_DISPLAY_RESULTS: usize = 500;

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Search,
    Universal,  // Flatpak search
    History,
    #[cfg(feature = "terraflow")]
    Audit,
}

/// Application state
pub struct App {
    /// Current mode
    pub mode: AppMode,
    /// Current search query
    pub query: String,
    /// Search result indices into the database
    pub results: Vec<usize>,
    /// Current selection index
    pub selected: usize,
    /// List widget state
    list_state: ListState,
    /// Current repository source filter
    pub source_filter: SourceFilter,
    /// UI theme
    pub theme: Theme,
    /// Arena-based package database
    pub database: PackageDatabase,
    /// Repository manager
    pub repo_manager: RepoManager,
    /// Installation history
    pub history: History,
    /// TerraFlow config (if detected)
    #[cfg(feature = "terraflow")]
    pub terraflow: Option<TerraFlow>,
    /// Audit results (cached)
    #[cfg(feature = "terraflow")]
    pub audit_result: Option<AuditResult>,
    /// Flatpak database (lazy loaded)
    pub flatpak: FlatpakDatabase,
    /// Flatpak search results
    pub flatpak_results: Vec<usize>,
    /// Status message
    pub status: String,
    /// Should quit
    pub should_quit: bool,
    /// Is loading
    pub is_loading: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFilter {
    All,
    Official,
    Aur,
}

impl SourceFilter {
    pub fn next(&self) -> Self {
        match self {
            SourceFilter::All => SourceFilter::Official,
            SourceFilter::Official => SourceFilter::Aur,
            SourceFilter::Aur => SourceFilter::All,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            SourceFilter::All => "ALL",
            SourceFilter::Official => "OFFICIAL",
            SourceFilter::Aur => "AUR",
        }
    }

    pub fn to_package_source(&self) -> Option<PackageSource> {
        match self {
            SourceFilter::All => None,
            SourceFilter::Official => Some(PackageSource::Official),
            SourceFilter::Aur => Some(PackageSource::Aur),
        }
    }
}

impl App {
    pub fn new() -> Self {
        let theme = Theme::load();
        let repo_manager = RepoManager::new();

        let mut app = Self {
            mode: AppMode::Search,
            query: String::new(),
            results: Vec::new(),
            selected: 0,
            list_state: ListState::default(),
            source_filter: SourceFilter::All,
            theme,
            database: PackageDatabase::new(),
            repo_manager,
            history: History::default(),
            #[cfg(feature = "terraflow")]
            terraflow: None,
            #[cfg(feature = "terraflow")]
            audit_result: None,
            flatpak: FlatpakDatabase::new(),
            flatpak_results: Vec::new(),
            status: String::from("Loading package database..."),
            should_quit: false,
            is_loading: true,
        };

        app.list_state.select(Some(0));
        app
    }

    /// Load the package database
    pub fn load_database(&mut self) {
        let start = Instant::now();
        self.database = PackageDatabase::load_or_build();

        let stats = &self.database.stats;
        let source = if stats.was_cached { "cache" } else { "pacman" };

        self.status = format!(
            "Loaded {} pkgs in {}ms ({})",
            stats.official_count + stats.aur_count,
            start.elapsed().as_millis(),
            source
        );
        self.is_loading = false;
    }

    /// Perform instant search
    pub fn search(&mut self) {
        if self.query.is_empty() {
            self.results.clear();
            self.status = format!("{} packages indexed", self.database.len());
            return;
        }

        if self.query.len() < 2 {
            self.results.clear();
            self.status = String::from("Type at least 2 chars...");
            return;
        }

        let start = Instant::now();
        self.results = self.database.search(
            &self.query,
            self.source_filter.to_package_source(),
            MAX_DISPLAY_RESULTS,
        );
        let elapsed_us = start.elapsed().as_micros();

        self.status = format!("Found {} in {}µs", self.results.len(), elapsed_us);
        self.selected = 0;
        self.list_state.select(Some(0));
    }

    /// Run TerraFlow audit
    #[cfg(feature = "terraflow")]
    pub fn run_audit(&mut self) {
        if let Some(ref tf) = self.terraflow {
            self.status = String::from("Running audit...");
            self.audit_result = Some(tf.audit());
            if let Some(ref result) = self.audit_result {
                self.status = format!(
                    "Audit: {} missing, {} extra",
                    result.missing.len(),
                    result.extra.len()
                );
            }
        } else {
            self.status = String::from("TerraFlow not configured");
        }
    }

    /// Switch to a different mode
    pub fn set_mode(&mut self, mode: AppMode) {
        self.mode = mode;
        self.selected = 0;
        self.list_state.select(Some(0));

        match mode {
            AppMode::Search => {
                self.status = format!("{} packages indexed", self.database.len());
            }
            AppMode::Universal => {
                self.load_flatpak();
            }
            AppMode::History => {
                self.status = format!(
                    "History: {} success, {} failed",
                    self.history.success_count(),
                    self.history.failure_count()
                );
            }
            #[cfg(feature = "terraflow")]
            AppMode::Audit => {
                self.run_audit();
            }
        }
    }

    /// Load Flatpak database on demand (lazy)
    pub fn load_flatpak(&mut self) {
        if !FlatpakDatabase::is_available() {
            self.status = String::from("Flatpak not installed");
            return;
        }

        if !self.flatpak.is_loaded() {
            self.status = String::from("Loading Flatpak database...");
            if let Err(e) = self.flatpak.load() {
                self.status = format!("Flatpak error: {}", e);
                return;
            }
        }

        let stats = &self.flatpak.stats;
        self.status = format!(
            "Flatpak: {} apps in {}ms ({})",
            stats.app_count, stats.load_time_ms, stats.source
        );
    }

    /// Search Flatpaks
    pub fn search_flatpak(&mut self) {
        if self.query.len() < 2 {
            self.flatpak_results.clear();
            self.status = String::from("Type at least 2 chars...");
            return;
        }

        let start = Instant::now();
        // Store indices for the results
        self.flatpak_results = (0..self.flatpak.len())
            .filter(|&idx| {
                let apps = self.flatpak.search(&self.query, MAX_DISPLAY_RESULTS);
                apps.iter().enumerate().any(|(i, _)| i == idx)
            })
            .take(MAX_DISPLAY_RESULTS)
            .collect();
        let elapsed_us = start.elapsed().as_micros();

        self.status = format!("Found {} Flatpaks in {}µs", self.flatpak_results.len(), elapsed_us);
        self.selected = 0;
        self.list_state.select(Some(0));
    }

    // Navigation methods
    pub fn select_previous(&mut self) {
        let len = match self.mode {
            AppMode::Search => self.results.len(),
            AppMode::Universal => self.flatpak.search(&self.query, MAX_DISPLAY_RESULTS).len(),
            AppMode::History => self.history.records.len(),
            #[cfg(feature = "terraflow")]
            AppMode::Audit => self.audit_result.as_ref().map(|r| r.missing.len()).unwrap_or(0),
        };
        if len == 0 {
            return;
        }
        self.selected = self.selected.saturating_sub(1);
        self.list_state.select(Some(self.selected));
    }

    pub fn select_next(&mut self) {
        let len = match self.mode {
            AppMode::Search => self.results.len(),
            AppMode::Universal => self.flatpak.search(&self.query, MAX_DISPLAY_RESULTS).len(),
            AppMode::History => self.history.records.len(),
            #[cfg(feature = "terraflow")]
            AppMode::Audit => self.audit_result.as_ref().map(|r| r.missing.len()).unwrap_or(0),
        };
        if len == 0 {
            return;
        }
        self.selected = (self.selected + 1).min(len.saturating_sub(1));
        self.list_state.select(Some(self.selected));
    }

    pub fn page_up(&mut self) {
        self.selected = self.selected.saturating_sub(10);
        self.list_state.select(Some(self.selected));
    }

    pub fn page_down(&mut self) {
        let len = match self.mode {
            AppMode::Search => self.results.len(),
            AppMode::Universal => self.flatpak.search(&self.query, MAX_DISPLAY_RESULTS).len(),
            AppMode::History => self.history.records.len(),
            #[cfg(feature = "terraflow")]
            AppMode::Audit => self.audit_result.as_ref().map(|r| r.missing.len()).unwrap_or(0),
        };
        self.selected = (self.selected + 10).min(len.saturating_sub(1));
        self.list_state.select(Some(self.selected));
    }

    pub fn selected_package(&self) -> Option<(&str, PackageSource)> {
        if self.mode != AppMode::Search {
            return None;
        }
        let idx = *self.results.get(self.selected)?;
        let name = self.database.get_name(idx)?;
        let source = self.database.get_source(idx)?;
        Some((name, source))
    }

    pub fn toggle_source(&mut self) {
        self.source_filter = self.source_filter.next();
        if self.mode == AppMode::Search {
            self.search();
        }
    }

    pub fn refresh_database(&mut self) {
        self.is_loading = true;
        self.status = String::from("Refreshing...");
        let _ = PackageDatabase::invalidate_cache();
        self.load_database();
        self.search();
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize terminal
pub fn init_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

/// Restore terminal
pub fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()
}

/// Main rendering function
pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(frame.area());

    draw_header(frame, chunks[0], app);

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(chunks[1]);

    match app.mode {
        AppMode::Search => {
            draw_package_list(frame, content_chunks[0], app);
            draw_preview(frame, content_chunks[1], app);
        }
        AppMode::Universal => {
            draw_flatpak_list(frame, content_chunks[0], app);
            draw_flatpak_preview(frame, content_chunks[1], app);
        }
        AppMode::History => {
            draw_history_list(frame, content_chunks[0], app);
            draw_history_detail(frame, content_chunks[1], app);
        }
        #[cfg(feature = "terraflow")]
        AppMode::Audit => {
            draw_audit_list(frame, content_chunks[0], app);
            draw_audit_detail(frame, content_chunks[1], app);
        }
    }

    draw_footer(frame, chunks[2], app);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let mode_label = match app.mode {
        AppMode::Search => format!("SEARCH | {}", app.source_filter.label()),
        AppMode::Universal => "UNIVERSAL (Flatpak)".to_string(),
        AppMode::History => "HISTORY".to_string(),
        #[cfg(feature = "terraflow")]
        AppMode::Audit => "AUDIT".to_string(),
    };

    let search_block = Block::default()
        .title(Span::styled(
            format!(" 🔍 TERRA STORE | {} ", mode_label),
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border));

    let content = if app.mode == AppMode::Search {
        let search_text = if app.query.is_empty() {
            Span::styled("Type to search...", Style::default().fg(theme.muted))
        } else {
            Span::styled(&app.query, Style::default().fg(theme.fg))
        };

        Line::from(vec![
            Span::styled("> ", Style::default().fg(theme.accent)),
            search_text,
            Span::styled("█", Style::default().fg(theme.accent)),
        ])
    } else {
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(theme.muted)),
            Span::styled("1", Style::default().fg(theme.accent)),
            Span::styled(" Search  ", Style::default().fg(theme.muted)),
            Span::styled("2", Style::default().fg(theme.accent)),
            Span::styled(" History  ", Style::default().fg(theme.muted)),
            Span::styled("3", Style::default().fg(theme.accent)),
            Span::styled(" Audit", Style::default().fg(theme.muted)),
        ])
    };

    let paragraph = Paragraph::new(content).block(search_block);
    frame.render_widget(paragraph, area);
}

fn draw_package_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;
    let visible_height = area.height.saturating_sub(2) as usize;
    let scroll_offset = app.selected.saturating_sub(visible_height / 2);
    let end_idx = (scroll_offset + visible_height).min(app.results.len());

    let items: Vec<ListItem> = app.results[scroll_offset..end_idx]
        .iter()
        .enumerate()
        .filter_map(|(i, &pkg_idx)| {
            let name = app.database.get_name(pkg_idx)?;
            let source = app.database.get_source(pkg_idx)?;
            let actual_idx = scroll_offset + i;

            let source_tag = match source {
                PackageSource::Official => Span::styled("[OFF]", Style::default().fg(theme.accent)),
                PackageSource::Aur => Span::styled("[AUR]", Style::default().fg(theme.secondary)),
            };

            let style = if actual_idx == app.selected {
                Style::default().bg(theme.highlight_bg).fg(theme.fg).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };

            Some(ListItem::new(Line::from(vec![
                source_tag,
                Span::raw(" "),
                Span::styled(name, style),
            ])))
        })
        .collect();

    let title = if app.is_loading {
        " Loading... ".to_string()
    } else {
        format!(" Results ({}) ", app.results.len())
    };

    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL).border_style(Style::default().fg(theme.border)))
        .highlight_style(Style::default().bg(theme.highlight_bg).add_modifier(Modifier::BOLD))
        .highlight_symbol("➜ ");

    let mut adjusted_state = ListState::default();
    if app.selected >= scroll_offset && app.selected < end_idx {
        adjusted_state.select(Some(app.selected - scroll_offset));
    }

    frame.render_stateful_widget(list, area, &mut adjusted_state);
}

fn draw_preview(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let content = if let Some((name, source)) = app.selected_package() {
        vec![
            Line::from(vec![
                Span::styled("📦 ", Style::default()),
                Span::styled(name, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Source: ", Style::default().fg(theme.muted)),
                match source {
                    PackageSource::Official => Span::styled("Official", Style::default().fg(theme.accent)),
                    PackageSource::Aur => Span::styled("AUR", Style::default().fg(theme.secondary)),
                },
            ]),
            Line::from(""),
            Line::from(Span::styled("Press Enter to install", Style::default().fg(theme.muted))),
        ]
    } else {
        let stats = &app.database.stats;
        vec![
            Line::from(Span::styled("Database Stats", Style::default().fg(theme.fg).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![
                Span::styled("Official: ", Style::default().fg(theme.muted)),
                Span::styled(format!("{}", stats.official_count), Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("AUR: ", Style::default().fg(theme.muted)),
                Span::styled(format!("{}", stats.aur_count), Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("Arena: ", Style::default().fg(theme.muted)),
                Span::styled(format!("{:.2} MB", stats.arena_bytes as f64 / 1_000_000.0), Style::default().fg(theme.fg)),
            ]),
        ]
    };

    let preview = Paragraph::new(content)
        .block(Block::default().title(" Details ").borders(Borders::ALL).border_style(Style::default().fg(theme.border)))
        .wrap(Wrap { trim: true });

    frame.render_widget(preview, area);
}

fn draw_flatpak_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;

    let results = app.flatpak.search(&app.query, MAX_DISPLAY_RESULTS);
    let visible_height = area.height.saturating_sub(2) as usize;
    let scroll_offset = app.selected.saturating_sub(visible_height / 2);
    let end_idx = (scroll_offset + visible_height).min(results.len());

    let items: Vec<ListItem> = if results.is_empty() && !app.flatpak.is_loaded() {
        vec![ListItem::new(Line::from(Span::styled(
            "Press F2 to load Flatpaks",
            Style::default().fg(theme.muted),
        )))]
    } else {
        results[scroll_offset..end_idx]
            .iter()
            .enumerate()
            .map(|(i, flatpak)| {
                let actual_idx = scroll_offset + i;
                let style = if actual_idx == app.selected {
                    Style::default().bg(theme.highlight_bg).fg(theme.fg).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg)
                };

                ListItem::new(Line::from(vec![
                    Span::styled("[FPK]", Style::default().fg(theme.secondary)),
                    Span::raw(" "),
                    Span::styled(&flatpak.name, style),
                ]))
            })
            .collect()
    };

    let title = format!(" Flatpak ({}) ", app.flatpak.len());

    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL).border_style(Style::default().fg(theme.border)))
        .highlight_style(Style::default().bg(theme.highlight_bg).add_modifier(Modifier::BOLD))
        .highlight_symbol("➜ ");

    let mut adjusted_state = ListState::default();
    if app.selected >= scroll_offset && app.selected < end_idx {
        adjusted_state.select(Some(app.selected - scroll_offset));
    }

    frame.render_stateful_widget(list, area, &mut adjusted_state);
}

fn draw_flatpak_preview(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let results = app.flatpak.search(&app.query, MAX_DISPLAY_RESULTS);
    let content = if let Some(flatpak) = results.get(app.selected) {
        vec![
            Line::from(vec![
                Span::styled("📦 ", Style::default()),
                Span::styled(&flatpak.name, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("ID: ", Style::default().fg(theme.muted)),
                Span::styled(&flatpak.id, Style::default().fg(theme.fg)),
            ]),
            Line::from(""),
            Line::from(Span::styled(&flatpak.summary, Style::default().fg(theme.fg))),
            Line::from(""),
            Line::from(Span::styled("Press Enter to install (flatpak)", Style::default().fg(theme.muted))),
        ]
    } else {
        let stats = &app.flatpak.stats;
        if app.flatpak.is_loaded() {
            vec![
                Line::from(Span::styled("Flatpak Database", Style::default().fg(theme.fg).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Apps: ", Style::default().fg(theme.muted)),
                    Span::styled(format!("{}", stats.app_count), Style::default().fg(theme.fg)),
                ]),
                Line::from(vec![
                    Span::styled("Source: ", Style::default().fg(theme.muted)),
                    Span::styled(&stats.source, Style::default().fg(theme.fg)),
                ]),
                Line::from(vec![
                    Span::styled("Load time: ", Style::default().fg(theme.muted)),
                    Span::styled(format!("{}ms", stats.load_time_ms), Style::default().fg(theme.fg)),
                ]),
            ]
        } else {
            vec![
                Line::from(Span::styled("Flatpak not loaded", Style::default().fg(theme.muted))),
                Line::from(""),
                Line::from(Span::styled("Type to search Flatpaks", Style::default().fg(theme.fg))),
            ]
        }
    };

    let preview = Paragraph::new(content)
        .block(Block::default().title(" Flatpak Details ").borders(Borders::ALL).border_style(Style::default().fg(theme.border)))
        .wrap(Wrap { trim: true });

    frame.render_widget(preview, area);
}

fn draw_history_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;
    let visible_height = area.height.saturating_sub(2) as usize;
    let records = app.history.recent(visible_height);

    let items: Vec<ListItem> = records
        .iter()
        .enumerate()
        .map(|(i, record)| {
            let status_icon = if record.success {
                Span::styled("✓", Style::default().fg(theme.success))
            } else {
                Span::styled("✗", Style::default().fg(theme.error))
            };

            let style = if i == app.selected {
                Style::default().bg(theme.highlight_bg).fg(theme.fg)
            } else {
                Style::default().fg(theme.fg)
            };

            ListItem::new(Line::from(vec![
                status_icon,
                Span::raw(" "),
                Span::styled(&record.name, style),
                Span::styled(format!(" ({})", record.formatted_time()), Style::default().fg(theme.muted)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title(format!(" History ({}) ", app.history.records.len())).borders(Borders::ALL).border_style(Style::default().fg(theme.border)))
        .highlight_symbol("➜ ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_history_detail(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let content = if let Some(record) = app.history.records.get(app.selected) {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("📦 ", Style::default()),
                Span::styled(&record.name, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Source: ", Style::default().fg(theme.muted)),
                Span::styled(format!("{}", record.source), Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("Time: ", Style::default().fg(theme.muted)),
                Span::styled(record.formatted_time(), Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(theme.muted)),
                if record.success {
                    Span::styled("Success", Style::default().fg(theme.success))
                } else {
                    Span::styled("Failed", Style::default().fg(theme.error))
                },
            ]),
        ];

        if let Some(ref error) = record.error {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("Error:", Style::default().fg(theme.error))));
            lines.push(Line::from(Span::styled(error, Style::default().fg(theme.muted))));
        }

        lines
    } else {
        vec![Line::from(Span::styled("No history selected", Style::default().fg(theme.muted)))]
    };

    let preview = Paragraph::new(content)
        .block(Block::default().title(" Details ").borders(Borders::ALL).border_style(Style::default().fg(theme.border)))
        .wrap(Wrap { trim: true });

    frame.render_widget(preview, area);
}

#[cfg(feature = "terraflow")]
fn draw_audit_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = &app.theme;

    let items: Vec<ListItem> = if let Some(ref result) = app.audit_result {
        result.missing.iter().enumerate().map(|(i, pkg)| {
            let source_tag = match pkg.source {
                PackageSource::Official => Span::styled("[OFF]", Style::default().fg(theme.accent)),
                PackageSource::Aur => Span::styled("[AUR]", Style::default().fg(theme.secondary)),
            };

            let style = if i == app.selected {
                Style::default().bg(theme.highlight_bg).fg(theme.fg)
            } else {
                Style::default().fg(theme.fg)
            };

            ListItem::new(Line::from(vec![
                source_tag,
                Span::raw(" "),
                Span::styled(&pkg.name, style),
            ]))
        }).collect()
    } else {
        vec![ListItem::new(Line::from(Span::styled("No audit data", Style::default().fg(theme.muted))))]
    };

    let title = if let Some(ref result) = app.audit_result {
        format!(" Missing ({}) ", result.missing.len())
    } else {
        " Audit ".to_string()
    };

    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL).border_style(Style::default().fg(theme.border)))
        .highlight_symbol("➜ ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

#[cfg(feature = "terraflow")]
fn draw_audit_detail(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let content = if let Some(ref result) = app.audit_result {
        if let Some(pkg) = result.missing.get(app.selected) {
            vec![
                Line::from(vec![
                    Span::styled("📦 ", Style::default()),
                    Span::styled(&pkg.name, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Source: ", Style::default().fg(theme.muted)),
                    Span::styled(format!("{}", pkg.source), Style::default().fg(theme.fg)),
                ]),
                Line::from(vec![
                    Span::styled("Config: ", Style::default().fg(theme.muted)),
                    Span::styled(&pkg.file, Style::default().fg(theme.fg)),
                ]),
                Line::from(""),
                Line::from(Span::styled("This package is in your config but not installed.", Style::default().fg(theme.error))),
            ]
        } else {
            vec![
                Line::from(Span::styled("Audit Summary", Style::default().fg(theme.fg).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Config packages: ", Style::default().fg(theme.muted)),
                    Span::styled(format!("{}", result.config_count), Style::default().fg(theme.fg)),
                ]),
                Line::from(vec![
                    Span::styled("Installed: ", Style::default().fg(theme.muted)),
                    Span::styled(format!("{}", result.installed_count), Style::default().fg(theme.fg)),
                ]),
                Line::from(vec![
                    Span::styled("Missing: ", Style::default().fg(theme.muted)),
                    Span::styled(format!("{}", result.missing.len()), Style::default().fg(theme.error)),
                ]),
            ]
        }
    } else {
        vec![
            Line::from(Span::styled("TerraFlow not configured", Style::default().fg(theme.muted))),
            Line::from(""),
            Line::from(Span::styled("Place package lists in:", Style::default().fg(theme.fg))),
            Line::from(Span::styled("  ~/TerraFlow-Dotfiles/packages/", Style::default().fg(theme.accent))),
        ]
    };

    let preview = Paragraph::new(content)
        .block(Block::default().title(" Details ").borders(Borders::ALL).border_style(Style::default().fg(theme.border)))
        .wrap(Wrap { trim: true });

    frame.render_widget(preview, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let keybindings = match app.mode {
        AppMode::Search => vec![
            Span::styled(" ↑↓", Style::default().fg(theme.accent)),
            Span::styled(" Nav ", Style::default().fg(theme.muted)),
            Span::styled("Enter", Style::default().fg(theme.accent)),
            Span::styled(" Install ", Style::default().fg(theme.muted)),
            Span::styled("Tab", Style::default().fg(theme.accent)),
            Span::styled(" Source ", Style::default().fg(theme.muted)),
            Span::styled("1-3", Style::default().fg(theme.accent)),
            Span::styled(" Mode ", Style::default().fg(theme.muted)),
            Span::styled("Esc", Style::default().fg(theme.accent)),
            Span::styled(" Quit", Style::default().fg(theme.muted)),
        ],
        AppMode::Universal => vec![
            Span::styled(" ↑↓", Style::default().fg(theme.accent)),
            Span::styled(" Nav ", Style::default().fg(theme.muted)),
            Span::styled("Enter", Style::default().fg(theme.accent)),
            Span::styled(" Install ", Style::default().fg(theme.muted)),
            Span::styled("F2", Style::default().fg(theme.accent)),
            Span::styled(" Reload ", Style::default().fg(theme.muted)),
            Span::styled("Esc", Style::default().fg(theme.accent)),
            Span::styled(" Quit", Style::default().fg(theme.muted)),
        ],
        AppMode::History => vec![
            Span::styled(" ↑↓", Style::default().fg(theme.accent)),
            Span::styled(" Nav ", Style::default().fg(theme.muted)),
            Span::styled("1-3", Style::default().fg(theme.accent)),
            Span::styled(" Mode ", Style::default().fg(theme.muted)),
            Span::styled("Esc", Style::default().fg(theme.accent)),
            Span::styled(" Quit", Style::default().fg(theme.muted)),
        ],
        #[cfg(feature = "terraflow")]
        AppMode::Audit => vec![
            Span::styled(" ↑↓", Style::default().fg(theme.accent)),
            Span::styled(" Nav ", Style::default().fg(theme.muted)),
            Span::styled("1-3", Style::default().fg(theme.accent)),
            Span::styled(" Mode ", Style::default().fg(theme.muted)),
            Span::styled("Esc", Style::default().fg(theme.accent)),
            Span::styled(" Quit", Style::default().fg(theme.muted)),
        ],
    };

    let status_style = if app.status.contains("µs") || app.status.contains("ms") {
        Style::default().fg(theme.success)
    } else if app.status.starts_with('✗') {
        Style::default().fg(theme.error)
    } else {
        Style::default().fg(theme.muted)
    };

    let footer_block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border));
    let footer_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(footer_block.inner(area));

    frame.render_widget(footer_block, area);
    frame.render_widget(Paragraph::new(Line::from(keybindings)), footer_layout[0]);
    frame.render_widget(Paragraph::new(Span::styled(&app.status, status_style)), footer_layout[1]);
}

/// Handle keyboard input
pub fn handle_input(app: &mut App) -> io::Result<bool> {
    if event::poll(Duration::from_millis(16))? {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(false);
            }

            match key.code {
                KeyCode::Esc => {
                    app.should_quit = true;
                    return Ok(true);
                }
                KeyCode::Char('1') => app.set_mode(AppMode::Search),
                KeyCode::F(2) => app.set_mode(AppMode::Universal),
                KeyCode::Char('2') => app.set_mode(AppMode::History),
                #[cfg(feature = "terraflow")]
                KeyCode::Char('3') => app.set_mode(AppMode::Audit),
                KeyCode::Up => app.select_previous(),
                KeyCode::Down => app.select_next(),
                KeyCode::PageUp => app.page_up(),
                KeyCode::PageDown => app.page_down(),
                KeyCode::Tab if app.mode == AppMode::Search => app.toggle_source(),
                KeyCode::F(5) if app.mode == AppMode::Search => app.refresh_database(),
                KeyCode::Enter if app.mode == AppMode::Search => {
                    if app.selected_package().is_some() {
                        return Ok(true);
                    }
                }
                KeyCode::Backspace if app.mode == AppMode::Search => {
                    app.query.pop();
                    app.search();
                }
                KeyCode::Backspace if app.mode == AppMode::Universal => {
                    app.query.pop();
                    app.search_flatpak();
                }
                KeyCode::Char(c) if app.mode == AppMode::Search => {
                    app.query.push(c);
                    app.search();
                }
                KeyCode::Char(c) if app.mode == AppMode::Universal => {
                    app.query.push(c);
                    app.search_flatpak();
                }
                _ => {}
            }
        }
    }

    Ok(false)
}
