use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use super::app::DashboardApp;

/// Poll timeout for event handling (250ms = 4 fps)
const POLL_TIMEOUT: Duration = Duration::from_millis(250);

/// Handle keyboard events for the dashboard
///
/// Returns Ok(()) on success, or an error if event reading fails.
/// This function is non-blocking - it polls with a timeout.
pub fn handle_events(app: &mut DashboardApp) -> io::Result<()> {
    // Poll for events with timeout (non-blocking)
    if event::poll(POLL_TIMEOUT)? {
        // Read the event
        if let Event::Key(key_event) = event::read()? {
            handle_key_event(app, key_event);
        }
    }
    Ok(())
}

/// Process a single key event
fn handle_key_event(app: &mut DashboardApp, key: KeyEvent) {
    if key.kind != event::KeyEventKind::Press {
        return;
    }

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
        KeyCode::Up => {
            if app.log_scroll > 0 {
                app.log_scroll = app.log_scroll.saturating_sub(1);
            }
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

    #[test]
    fn test_ignore_other_keys() {
        let mut app = create_test_app();
        let initial_state = (app.should_quit, app.dashboard_visible, app.log_scroll);

        // Test various keys that should be ignored
        let keys = vec![
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        ];

        for key in keys {
            handle_key_event(&mut app, key);
        }

        // State should be unchanged
        assert_eq!(
            (app.should_quit, app.dashboard_visible, app.log_scroll),
            initial_state
        );
    }
}
