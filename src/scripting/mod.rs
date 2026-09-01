//! Lua scripting integration layer for BPC.
//!
//! Provides [`ScriptEngine`], an embeddable Lua runtime that exposes BPC's
//! layout and bit-reading APIs as a Lua DSL. Users can declare protocol
//! layouts, resolve variable-width fields against data, and read field
//! values—all from Lua scripts.
//!
//! ## Design
//!
//! The scripting layer is intentionally extensible. The [`ScriptEngine`]
//! initializes a Lua VM and registers a `bpc` module table with functions
//! and types. New capabilities (test harnesses, format hooks, protocol
//! validators) can be added by registering additional Lua functions or
//! UserData types on the same engine.
//!
//! ## Lua DSL Overview
//!
//! ```lua
//! local bpc = require("bpc")
//!
//! -- Declare a layout
//! local layout = bpc.layout("my_protocol")
//!     :field("version", 4)
//!     :field("type", 4)
//!     :field("length", 8)
//!     :field_var("payload", "length", "bytes")
//!     :build()
//!
//! -- Resolve against data
//! local data = bpc.hex("04 03 AA BB CC")
//! local resolved = layout:resolve(data)
//!
//! -- Query fields
//! print(resolved:field("version").width)    -- 4
//! print(resolved:field("payload").width)    -- 24
//! print(resolved:bit_len())                -- 36
//! ```

pub mod engine;
pub mod error;

pub use engine::ScriptEngine;
pub use error::ScriptError;
