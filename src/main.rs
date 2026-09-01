use bpc::layout::BitLayout;
use bpc::tui::{App, LayoutViewState};
use std::io;

fn main() -> io::Result<()> {
    // Set up a panic hook that restores the terminal before printing the panic.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = ratatui::restore();
        original_hook(panic_info);
    }));

    let mut terminal = ratatui::init();

    // Demo layout: a simple 32-bit packet header.
    let layout = BitLayout::builder()
        .field("version", 3)
        .field("opcode", 5)
        .field("length", 16)
        .field("flags", 8)
        .build()
        .expect("demo layout should be valid");

    let data = vec![0b10110100, 0b01100001, 0b11110000, 0b10100010];

    let mut app = App::new();
    app.set_layout_view(LayoutViewState::new(layout, data));

    let result = app.run(&mut terminal);
    ratatui::restore();
    result
}
