# BPC (Binary Protocol Workbench)

BPC is a terminal-based workbench for inspecting, crafting, and testing custom binary protocols.

The goal is to provide a bit-level engine combined with a scriptable environment (Lua) so you can parse arbitrary binary formats, construct packets on the fly, and interact with live network/serial endpoints.

## Current Status

**Phase 0 (Foundation)** — The project layout, library/binary split, TUI shell, and CI pipeline are set up. No protocol engine or bit manipulation logic is implemented yet.

## Roadmap

- [x] **Phase 0** — Foundation & TUI skeleton
- [x] **Phase 1** — Bit reader / writer engine
- [x] **Phase 2** — Bit layout & field definitions
- [x] **Phase 3** — Typed values & conversions
- [x] **Phase 4** — Protocol schema & message specs
- [x] **Phase 5** — Lua scripting integration
- [x] **Phase 6** — Interactive TUI workbench
- [ ] **Phase 7** — Transport drivers (TCP/UDP/Serial)
- [ ] **Phase 8** — Protocol testing & fuzzing framework

## Development

Requires Rust (2024 edition).

```bash
# Build binary & library
cargo build

# Run the TUI (defaults to a built-in demo)
cargo run

# Run test suite
cargo test
```

## Examples

BPC supports loading protocol layouts via Lua scripts and rendering them against binary data files. We have included several real-world binary protocol examples in the `examples/` directory.

You can try them out by running:

```bash
# HTTP/2 Frame (demonstrates dynamic payload parsing)
cargo run -- examples/http2_frame.lua examples/http2_frame.bin

# IPv6 Packet Header (demonstrates 128-bit fields and dynamic payloads)
cargo run -- examples/ipv6_packet.lua examples/ipv6_packet.bin

# TCP Header (demonstrates dense bit-packing and boolean flags)
cargo run -- examples/tcp_header.lua examples/tcp_header.bin
```

## Note

Doc comments in this codebase are AI-generated.

## License

[MIT](LICENSE)
