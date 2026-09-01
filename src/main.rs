use bpc::layout::BitLayout;
use bpc::scripting::ScriptEngine;
use bpc::tui::{App, LayoutViewState};
use std::env;
use std::fs;
use std::io;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let (layout, data) = if args.len() == 3 {
        let script_path = &args[1];
        let data_path = &args[2];

        // 1. Read the binary data file
        let data = fs::read(data_path)?;

        // 2. Initialize the script engine and execute the Lua file
        let engine = ScriptEngine::new().expect("Failed to initialize script engine");
        engine
            .exec_file(std::path::Path::new(script_path))
            .unwrap_or_else(|e| panic!("Failed to execute script '{}': {}", script_path, e));

        // 3. Extract the layout from a global variable named `layout`
        let raw_layout = engine
            .get_global_layout("layout")
            .expect("Script must define a global variable named `layout` (don't forget to call :build())");

        // 4. Resolve the layout against the data if there are variable-width fields
        let resolved_layout = if raw_layout.has_variable_fields() {
            raw_layout
                .resolve(&data)
                .expect("Failed to resolve variable-width fields with the provided data")
        } else {
            raw_layout
        };

        (resolved_layout, data)
    } else {
        println!("Usage: bpc <layout.lua> <data.bin>");
        println!("Falling back to built-in demo layout...\n");

        // Demo layout fallback
        let layout = BitLayout::builder()
            .field("version", 3)
            .field("opcode", 5)
            .field("length", 16)
            .field("flags", 8)
            .build()
            .unwrap();

        let data = vec![0b10110100, 0b01100001, 0b11110000, 0b10100010];
        (layout, data)
    };

    // Set up a panic hook that restores the terminal before printing the panic.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = ratatui::restore();
        original_hook(panic_info);
    }));

    let mut terminal = ratatui::init();

    let mut app = App::new();
    app.set_layout_view(LayoutViewState::new(layout, data));

    let result = app.run(&mut terminal);
    ratatui::restore();
    result
}
