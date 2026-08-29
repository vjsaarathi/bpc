# BPC (Binary Protocol Workbench)

BPC is a terminal-based workbench for inspecting, crafting, and testing custom binary protocols.

The goal is to provide a bit-level engine combined with a scriptable environment (Lua) so you can parse arbitrary binary formats, construct packets on the fly, and interact with live network/serial endpoints.

## Current Status

**Phase 0 (Foundation)** — The project layout, library/binary split, TUI shell, and CI pipeline are set up. No protocol engine or bit manipulation logic is implemented yet.

## Roadmap

- [x] **Phase 0** — Foundation & TUI skeleton
- [ ] **Phase 1** — Bit reader / writer engine
- [ ] **Phase 2** — Bit layout & field definitions
- [ ] **Phase 3** — Typed values & conversions
- [ ] **Phase 4** — Protocol schema & message specs
- [ ] **Phase 5** — Lua scripting integration
- [ ] **Phase 6** — Interactive TUI workbench
- [ ] **Phase 7** — Transport drivers (TCP/UDP/Serial)
- [ ] **Phase 8** — Protocol testing & fuzzing framework

## Development

Requires Rust (2024 edition).

```bash
# Build binary & library
cargo build

# Run the TUI
cargo run

# Run test suite
cargo test
```

## Note

Doc comments in this codebase are AI-generated.

## License

[MIT](LICENSE)
