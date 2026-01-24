use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use tui_input::backend::crossterm::EventHandler;

use super::app::{DashboardApp, InputMode};

const POLL_TIMEOUT: Duration = Duration::from_millis(250);

pub fn handle_events(app: &mut DashboardApp) -> io::Result<()> {
    if event::poll(POLL_TIMEOUT)? {
        if let Event::Key(key_event) = event::read()? {
            handle_key_event(app, key_event);
        }
    }
    Ok(())
}

fn handle_key_event(app: &mut DashboardApp, key: KeyEvent) {
    if key.kind != event::KeyEventKind::Press {
        return;
    }

    match app.input_mode {
        InputMode::Normal => handle_normal_mode(app, key),
        InputMode::Search => handle_search_mode(app, key),
    }
}

fn handle_normal_mode(app: &mut DashboardApp, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::Char('d') => {
            app.dashboard_visible = !app.dashboard_visible;
        }
        KeyCode::Char('/') => {
            app.input_mode = InputMode::Search;
        }
        KeyCode::Char('s') => {
            app.show_session_view = !app.show_session_view;
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            app.increase_log_height();
        }
        KeyCode::Char('-') | KeyCode::Char('_') => {
            app.decrease_log_height();
        }
        KeyCode::Char('[') => {
            app.decrease_middle_height();
        }
        KeyCode::Char(']') => {
            app.increase_middle_height();
        }
        KeyCode::Up => {
            app.log_scroll = app.log_scroll.saturating_sub(1);
        }
        KeyCode::Down => {
            app.log_scroll = app.log_scroll.saturating_add(1);
        }
        KeyCode::PageUp => {
            app.log_scroll = app.log_scroll.saturating_sub(10);
        }
        KeyCode::PageDown => {
            app.log_scroll = app.log_scroll.saturating_add(10);
        }
        KeyCode::Home => {
            app.log_scroll = 0;
        }
        _ => {}
    }
}

fn handle_search_mode(app: &mut DashboardApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            app.apply_search();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Esc => {
            app.clear_search();
            app.input_mode = InputMode::Normal;
        }
        _ => {
            app.search_input.handle_event(&Event::Key(key));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard::app::DashboardApp;
    use crate::metrics::MetricsCollector;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    fn create_test_app() -> DashboardApp {
        let metrics = Arc::new(MetricsCollector::new());
        let log_buffer = Arc::new(Mutex::new(VecDeque::new()));
        DashboardApp::new(metrics, log_buffer)
    }

    #[test]
    fn test_quit_key() {
        let mut app = create_test_app();
        assert!(!app.should_quit);

        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        handle_key_event(&mut app, key);

        assert!(app.should_quit);
    }

    #[test]
    fn test_escape_key() {
        let mut app = create_test_app();
        assert!(!app.should_quit);

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        handle_key_event(&mut app, key);

        assert!(app.should_quit);
    }

    #[test]
    fn test_ctrl_c() {
        let mut app = create_test_app();
        assert!(!app.should_quit);

        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        handle_key_event(&mut app, key);

        assert!(app.should_quit);
    }

    #[test]
    fn test_toggle_dashboard() {
        let mut app = create_test_app();
        assert!(app.dashboard_visible);

        let key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        handle_key_event(&mut app, key);

        assert!(!app.dashboard_visible);

        handle_key_event(&mut app, key);
        assert!(app.dashboard_visible);
    }

    #[test]
    fn test_search_mode_toggle() {
        let mut app = create_test_app();
        assert_eq!(app.input_mode, InputMode::Normal);

        let key = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
        handle_key_event(&mut app, key);

        assert_eq!(app.input_mode, InputMode::Search);
    }

    #[test]
    fn test_session_view_toggle() {
        let mut app = create_test_app();
        assert!(!app.show_session_view);

        let key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        handle_key_event(&mut app, key);

        assert!(app.show_session_view);
    }

    #[test]
    fn test_scroll_up() {
        let mut app = create_test_app();
        app.log_scroll = 5;

        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        handle_key_event(&mut app, key);

        assert_eq!(app.log_scroll, 4);
    }

    #[test]
    fn test_scroll_down() {
        let mut app = create_test_app();
        app.log_scroll = 5;

        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        handle_key_event(&mut app, key);

        assert_eq!(app.log_scroll, 6);
    }

    #[test]
    fn test_scroll_up_at_zero() {
        let mut app = create_test_app();
        app.log_scroll = 0;

        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        handle_key_event(&mut app, key);

        assert_eq!(app.log_scroll, 0);
    }

    #[test]
    fn test_page_up() {
        let mut app = create_test_app();
        app.log_scroll = 15;

        let key = KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE);
        handle_key_event(&mut app, key);

        assert_eq!(app.log_scroll, 5);
    }

    #[test]
    fn test_page_down() {
        let mut app = create_test_app();
        app.log_scroll = 5;

        let key = KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE);
        handle_key_event(&mut app, key);

        assert_eq!(app.log_scroll, 15);
    }

    #[test]
    fn test_home_key() {
        let mut app = create_test_app();
        app.log_scroll = 100;

        let key = KeyEvent::new(KeyCode::Home, KeyModifiers::NONE);
        handle_key_event(&mut app, key);

        assert_eq!(app.log_scroll, 0);
    }

    #[test]
    fn test_page_up_at_zero() {
        let mut app = create_test_app();
        app.log_scroll = 5;

        let key = KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE);
        handle_key_event(&mut app, key);

        assert_eq!(app.log_scroll, 0);
    }
}
