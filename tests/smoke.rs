//! Integration tests for BPC.

#[test]
fn library_is_usable() {
    // Verify the library crate links and the App type is accessible.
    let app = bpc::tui::App::new();
    assert!(app.is_running());
}

#[test]
fn app_can_quit() {
    let mut app = bpc::tui::App::new();
    app.quit();
    assert!(!app.is_running());
}
