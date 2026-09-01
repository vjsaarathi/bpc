//! The Lua script engine — BPC's extensible scripting runtime.
//!
//! [`ScriptEngine`] wraps a Lua VM and exposes BPC's core types and
//! operations as Lua functions and UserData. It is the single entry point
//! for running user scripts.

use std::cell::RefCell;
use std::rc::Rc;

use mlua::{FromLua, IntoLua, Lua, Result as LuaResult, UserData, UserDataMethods, Value};

use crate::bit::BitReader;
use crate::layout::{BitLayout, BitLayoutBuilder, LengthUnit};

use super::error::ScriptError;

// ---------------------------------------------------------------------------
// Lua UserData wrappers
// ---------------------------------------------------------------------------

/// Shared builder state that both [`LuaLayoutBuilder`] and
/// [`LuaLayoutBuilderRef`] point to, enabling method chaining.
type SharedBuilder = Rc<RefCell<Option<BitLayoutBuilder>>>;

/// Lua-side representation of a layout builder (the in-progress chain).
///
/// Created by `bpc.layout(name)` and consumed by `:build()`.
struct LuaLayoutBuilder {
    name: String,
    builder: SharedBuilder,
}

/// A thin clone-handle so method chaining works (`:field(...):field(...)`).
///
/// This struct shares the same underlying [`SharedBuilder`] as the original
/// [`LuaLayoutBuilder`], so mutations propagate.
struct LuaLayoutBuilderRef {
    name: String,
    builder: SharedBuilder,
}

/// Adds the common builder methods to any UserData type that carries
/// `name: String` and `builder: SharedBuilder`.
macro_rules! impl_builder_methods {
    ($methods:ident) => {
        // :field(name, width) -> self
        $methods.add_method_mut("field", |_, this, (name, width): (String, usize)| {
            let mut inner = this.builder.borrow_mut();
            let b = inner
                .take()
                .ok_or_else(|| mlua::Error::runtime("builder already consumed by :build()"))?;
            *inner = Some(b.field(&name, width));
            Ok(LuaLayoutBuilderRef {
                name: this.name.clone(),
                builder: Rc::clone(&this.builder),
            })
        });

        // :field_var(name, source_field, unit_str) -> self
        $methods.add_method_mut(
            "field_var",
            |_, this, (name, source, unit_str): (String, String, String)| {
                let unit = parse_length_unit(&unit_str)?;
                let mut inner = this.builder.borrow_mut();
                let b = inner.take().ok_or_else(|| {
                    mlua::Error::runtime("builder already consumed by :build()")
                })?;
                *inner = Some(b.field_var(&name, &source, unit));
                Ok(LuaLayoutBuilderRef {
                    name: this.name.clone(),
                    builder: Rc::clone(&this.builder),
                })
            },
        );

        // :field_at(name, offset, width) -> self
        $methods.add_method_mut(
            "field_at",
            |_, this, (name, offset, width): (String, usize, usize)| {
                let mut inner = this.builder.borrow_mut();
                let b = inner.take().ok_or_else(|| {
                    mlua::Error::runtime("builder already consumed by :build()")
                })?;
                *inner = Some(b.field_at(&name, offset, width));
                Ok(LuaLayoutBuilderRef {
                    name: this.name.clone(),
                    builder: Rc::clone(&this.builder),
                })
            },
        );

        // :build() -> LuaLayout
        $methods.add_method_mut("build", |_, this, ()| {
            let mut inner = this.builder.borrow_mut();
            let b = inner
                .take()
                .ok_or_else(|| mlua::Error::runtime("builder already consumed by :build()"))?;
            let layout = b
                .build()
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;
            Ok(LuaLayout {
                name: this.name.clone(),
                layout,
            })
        });
    };
}

impl UserData for LuaLayoutBuilder {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        impl_builder_methods!(methods);
    }
}

impl UserData for LuaLayoutBuilderRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        impl_builder_methods!(methods);
    }
}

/// Lua-side representation of a built [`BitLayout`].
///
/// Exposes querying and resolution methods.
#[derive(Clone)]
struct LuaLayout {
    name: String,
    layout: BitLayout,
}

impl UserData for LuaLayout {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // :bit_len() -> integer
        methods.add_method("bit_len", |_, this, ()| Ok(this.layout.bit_len()));

        // :field_count() -> integer
        methods.add_method("field_count", |_, this, ()| Ok(this.layout.field_count()));

        // :has_variable_fields() -> boolean
        methods.add_method("has_variable_fields", |_, this, ()| {
            Ok(this.layout.has_variable_fields())
        });

        // :name() -> string
        methods.add_method("name", |_, this, ()| Ok(this.name.clone()));

        // :field(name_or_index) -> LuaFieldInfo | nil
        methods.add_method("field", |_, this, key: Value| match key {
            Value::Integer(idx) => {
                let idx = (idx - 1) as usize; // Lua is 1-indexed
                Ok(this.layout.field(idx).map(field_to_lua_info))
            }
            Value::String(s) => {
                let name = s.to_str().map_err(|e| mlua::Error::runtime(e.to_string()))?;
                Ok(this.layout.field_by_name(&name).map(field_to_lua_info))
            }
            _ => Err(mlua::Error::runtime(
                "field() expects a string name or integer index",
            )),
        });

        // :fields() -> table of LuaFieldInfo
        methods.add_method("fields", |lua, this, ()| {
            let table = lua.create_table()?;
            for (i, f) in this.layout.fields().iter().enumerate() {
                table.set(i + 1, field_to_lua_info(f))?;
            }
            Ok(table)
        });

        // :resolve(data_bytes) -> LuaLayout
        methods.add_method("resolve", |_, this, data: Vec<u8>| {
            let resolved = this
                .layout
                .resolve(&data)
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;
            Ok(LuaLayout {
                name: this.name.clone(),
                layout: resolved,
            })
        });

        // :read_field(field_name, data_bytes) -> integer | nil
        methods.add_method("read_field", |_, this, (name, data): (String, Vec<u8>)| {
            let field = this
                .layout
                .field_by_name(&name)
                .ok_or_else(|| mlua::Error::runtime(format!("unknown field \"{name}\"")))?;

            if field.width() == 0 || field.width() > 64 {
                return Ok(Value::Nil);
            }
            if field.end() > data.len() * 8 {
                return Ok(Value::Nil);
            }

            let mut reader = BitReader::from_bytes(&data);
            reader
                .skip(field.offset())
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;
            let val = reader
                .read_bits(field.width() as u32)
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;
            Ok(Value::Integer(val as i64))
        });

        // :field_at_bit(bit_offset) -> LuaFieldInfo | nil
        methods.add_method("field_at_bit", |_, this, offset: usize| {
            Ok(this.layout.field_at_bit(offset).map(field_to_lua_info))
        });
    }
}

/// Lua-side field info, exposed as a UserData with getter methods.
///
/// ```lua
/// local f = layout:field("version")
/// print(f:get_name())       -- "version"
/// print(f:get_width())      -- 4
/// print(f:get_offset())     -- 0
/// print(f:get_end())        -- 4
/// print(f:is_variable())    -- false
/// ```
#[derive(Clone)]
struct LuaFieldInfo {
    name: String,
    offset: usize,
    width: usize,
    end: usize,
    is_variable: bool,
}

impl UserData for LuaFieldInfo {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get_name", |_, this, ()| Ok(this.name.clone()));
        methods.add_method("get_offset", |_, this, ()| Ok(this.offset));
        methods.add_method("get_width", |_, this, ()| Ok(this.width));
        methods.add_method("get_end", |_, this, ()| Ok(this.end));
        methods.add_method("is_variable", |_, this, ()| Ok(this.is_variable));
    }
}

/// Converts a [`LayoutField`] to a [`LuaFieldInfo`].
fn field_to_lua_info(f: &crate::layout::LayoutField) -> LuaFieldInfo {
    LuaFieldInfo {
        name: f.name().to_string(),
        offset: f.offset(),
        width: f.width(),
        end: f.end(),
        is_variable: f.is_variable(),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_length_unit(s: &str) -> LuaResult<LengthUnit> {
    match s.to_lowercase().as_str() {
        "bits" | "bit" | "b" => Ok(LengthUnit::Bits),
        "bytes" | "byte" => Ok(LengthUnit::Bytes),
        _ => Err(mlua::Error::runtime(format!(
            "unknown length unit \"{s}\": expected \"bits\" or \"bytes\""
        ))),
    }
}

// ---------------------------------------------------------------------------
// ScriptEngine
// ---------------------------------------------------------------------------

/// The BPC Lua scripting engine.
///
/// Wraps a Lua VM with the `bpc` module pre-registered. Use [`eval`](Self::eval)
/// and [`exec`](Self::exec) to run Lua code, or [`exec_file`](Self::exec_file)
/// to run scripts from disk.
///
/// # Examples
///
/// ```
/// use bpc::scripting::ScriptEngine;
///
/// let engine = ScriptEngine::new().unwrap();
/// let result: i64 = engine.eval("return 1 + 2").unwrap();
/// assert_eq!(result, 3);
/// ```
pub struct ScriptEngine {
    lua: Lua,
}

impl ScriptEngine {
    /// Creates a new scripting engine with the `bpc` module registered.
    pub fn new() -> Result<Self, ScriptError> {
        let lua = Lua::new();
        register_bpc_module(&lua)?;
        Ok(Self { lua })
    }

    /// Evaluates a Lua expression and returns the result.
    pub fn eval<T: FromLua>(&self, code: &str) -> Result<T, ScriptError> {
        let val = self.lua.load(code).eval::<T>()?;
        Ok(val)
    }

    /// Executes Lua code without returning a value.
    pub fn exec(&self, code: &str) -> Result<(), ScriptError> {
        self.lua.load(code).exec()?;
        Ok(())
    }

    /// Executes a Lua script file.
    pub fn exec_file(&self, path: &std::path::Path) -> Result<(), ScriptError> {
        let code = std::fs::read_to_string(path)
            .map_err(|e| ScriptError::Message(format!("cannot read {}: {e}", path.display())))?;
        self.lua
            .load(&code)
            .set_name(path.to_string_lossy())
            .exec()?;
        Ok(())
    }

    /// Returns a reference to the underlying Lua instance.
    ///
    /// Useful for advanced integrations that need to register additional
    /// globals or UserData types.
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// Sets a global variable visible to Lua scripts.
    pub fn set_global<T: IntoLua>(&self, name: &str, value: T) -> Result<(), ScriptError> {
        self.lua.globals().set(name, value)?;
        Ok(())
    }

    /// Gets a global variable from the Lua environment.
    pub fn get_global<T: FromLua>(&self, name: &str) -> Result<T, ScriptError> {
        let val = self.lua.globals().get::<T>(name)?;
        Ok(val)
    }
}

/// Registers the `bpc` module into the Lua `package.loaded` table.
fn register_bpc_module(lua: &Lua) -> LuaResult<()> {
    let bpc = lua.create_table()?;

    // bpc.layout(name) -> LuaLayoutBuilder
    let layout_fn = lua.create_function(|_, name: String| {
        Ok(LuaLayoutBuilder {
            name,
            builder: Rc::new(RefCell::new(Some(BitLayout::builder()))),
        })
    })?;
    bpc.set("layout", layout_fn)?;

    // bpc.hex(hex_string) -> table of bytes
    //
    // Parses a hex string like "04 03 AA BB CC" or "0403AABBCC"
    // into a byte array.
    let hex_fn = lua.create_function(|_, hex_str: String| {
        let cleaned: String = hex_str
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect();
        if cleaned.len() % 2 != 0 {
            return Err(mlua::Error::runtime(
                "hex string must have an even number of hex digits",
            ));
        }
        let bytes: Vec<u8> = (0..cleaned.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&cleaned[i..i + 2], 16).unwrap())
            .collect();
        Ok(bytes)
    })?;
    bpc.set("hex", hex_fn)?;

    // bpc.bytes(...) -> byte array
    //
    // Convenience for creating a byte array from integer arguments.
    let bytes_fn = lua.create_function(|_, args: mlua::Variadic<u8>| {
        let bytes: Vec<u8> = args.into_iter().collect();
        Ok(bytes)
    })?;
    bpc.set("bytes", bytes_fn)?;

    // Register as a preloaded module so `require("bpc")` works.
    let package: mlua::Table = lua.globals().get("package")?;
    let loaded: mlua::Table = package.get("loaded")?;
    loaded.set("bpc", bpc.clone())?;

    // Also set as a global for convenience.
    lua.globals().set("bpc", bpc)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> ScriptEngine {
        ScriptEngine::new().expect("engine creation should succeed")
    }

    // -- Basic engine tests --

    #[test]
    fn eval_simple_expression() {
        let e = engine();
        let result: i64 = e.eval("return 1 + 2").unwrap();
        assert_eq!(result, 3);
    }

    #[test]
    fn exec_sets_global() {
        let e = engine();
        e.exec("x = 42").unwrap();
        let x: i64 = e.get_global("x").unwrap();
        assert_eq!(x, 42);
    }

    #[test]
    fn set_and_get_global() {
        let e = engine();
        e.set_global("greeting", "hello").unwrap();
        let g: String = e.get_global("greeting").unwrap();
        assert_eq!(g, "hello");
    }

    // -- bpc.hex --

    #[test]
    fn hex_parses_spaced_string() {
        let e = engine();
        let result: Vec<u8> = e.eval(r#"return bpc.hex("04 03 AA BB CC")"#).unwrap();
        assert_eq!(result, vec![0x04, 0x03, 0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn hex_parses_compact_string() {
        let e = engine();
        let result: Vec<u8> = e.eval(r#"return bpc.hex("DEADBEEF")"#).unwrap();
        assert_eq!(result, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn hex_rejects_odd_digits() {
        let e = engine();
        let result = e.eval::<Vec<u8>>(r#"return bpc.hex("ABC")"#);
        assert!(result.is_err());
    }

    // -- bpc.bytes --

    #[test]
    fn bytes_creates_array() {
        let e = engine();
        let result: Vec<u8> = e.eval("return bpc.bytes(1, 2, 3, 255)").unwrap();
        assert_eq!(result, vec![1, 2, 3, 255]);
    }

    // -- Layout builder DSL --

    #[test]
    fn layout_fixed_fields() {
        let e = engine();
        e.exec(
            r#"
            local bpc = require("bpc")
            layout = bpc.layout("test_proto")
                :field("version", 4)
                :field("type", 4)
                :field("length", 8)
                :build()
        "#,
        )
        .unwrap();

        let bit_len: i64 = e.eval("return layout:bit_len()").unwrap();
        assert_eq!(bit_len, 16);

        let count: i64 = e.eval("return layout:field_count()").unwrap();
        assert_eq!(count, 3);

        let name: String = e.eval("return layout:name()").unwrap();
        assert_eq!(name, "test_proto");
    }

    #[test]
    fn layout_field_query_by_name() {
        let e = engine();
        e.exec(
            r#"
            layout = bpc.layout("q")
                :field("ver", 3)
                :field("op", 5)
                :build()
            f = layout:field("ver")
        "#,
        )
        .unwrap();

        let name: String = e.eval("return f:get_name()").unwrap();
        assert_eq!(name, "ver");
        let width: i64 = e.eval("return f:get_width()").unwrap();
        assert_eq!(width, 3);
        let offset: i64 = e.eval("return f:get_offset()").unwrap();
        assert_eq!(offset, 0);
    }

    #[test]
    fn layout_field_query_by_index() {
        let e = engine();
        e.exec(
            r#"
            layout = bpc.layout("idx")
                :field("a", 8)
                :field("b", 16)
                :build()
            f = layout:field(2)  -- Lua 1-indexed
        "#,
        )
        .unwrap();

        let name: String = e.eval("return f:get_name()").unwrap();
        assert_eq!(name, "b");
        let offset: i64 = e.eval("return f:get_offset()").unwrap();
        assert_eq!(offset, 8);
    }

    #[test]
    fn layout_fields_list() {
        let e = engine();
        e.exec(
            r#"
            layout = bpc.layout("list")
                :field("a", 8)
                :field("b", 8)
                :build()
            fs = layout:fields()
        "#,
        )
        .unwrap();

        let len: i64 = e.eval("return #fs").unwrap();
        assert_eq!(len, 2);
        let name1: String = e.eval("return fs[1]:get_name()").unwrap();
        assert_eq!(name1, "a");
        let name2: String = e.eval("return fs[2]:get_name()").unwrap();
        assert_eq!(name2, "b");
    }

    // -- Variable-width fields via Lua --

    #[test]
    fn layout_var_field_and_resolve() {
        let e = engine();
        e.exec(
            r#"
            local bpc = require("bpc")
            template = bpc.layout("var_proto")
                :field("length", 8)
                :field_var("payload", "length", "bytes")
                :build()
        "#,
        )
        .unwrap();

        let has_var: bool = e.eval("return template:has_variable_fields()").unwrap();
        assert!(has_var);

        // Resolve with length=3 → payload=24 bits
        e.exec(
            r#"
            data = bpc.hex("03 AA BB CC")
            resolved = template:resolve(data)
        "#,
        )
        .unwrap();

        let bit_len: i64 = e.eval("return resolved:bit_len()").unwrap();
        assert_eq!(bit_len, 32);

        let has_var_resolved: bool =
            e.eval("return resolved:has_variable_fields()").unwrap();
        assert!(!has_var_resolved);

        let pw: i64 =
            e.eval("return resolved:field('payload'):get_width()").unwrap();
        assert_eq!(pw, 24);

        let po: i64 = e
            .eval("return resolved:field('payload'):get_offset()")
            .unwrap();
        assert_eq!(po, 8);
    }

    #[test]
    fn layout_var_field_bits_unit() {
        let e = engine();
        e.exec(
            r#"
            template = bpc.layout("bits_proto")
                :field("bit_count", 8)
                :field_var("data", "bit_count", "bits")
                :build()
            resolved = template:resolve(bpc.hex("0C FF FF"))
        "#,
        )
        .unwrap();

        // 0x0C = 12, so data = 12 bits
        let dw: i64 = e.eval("return resolved:field('data'):get_width()").unwrap();
        assert_eq!(dw, 12);
    }

    // -- read_field --

    #[test]
    fn read_field_value() {
        let e = engine();
        e.exec(
            r#"
            layout = bpc.layout("read_test")
                :field("a", 8)
                :field("b", 8)
                :build()
            data = bpc.hex("AB CD")
        "#,
        )
        .unwrap();

        let a: i64 = e.eval("return layout:read_field('a', data)").unwrap();
        assert_eq!(a, 0xAB);
        let b: i64 = e.eval("return layout:read_field('b', data)").unwrap();
        assert_eq!(b, 0xCD);
    }

    // -- field_at_bit --

    #[test]
    fn field_at_bit_query() {
        let e = engine();
        e.exec(
            r#"
            layout = bpc.layout("at_bit")
                :field("a", 8)
                :field("b", 8)
                :build()
        "#,
        )
        .unwrap();

        let name: String =
            e.eval("return layout:field_at_bit(0):get_name()").unwrap();
        assert_eq!(name, "a");

        let name: String =
            e.eval("return layout:field_at_bit(8):get_name()").unwrap();
        assert_eq!(name, "b");

        let is_nil: bool =
            e.eval("return layout:field_at_bit(16) == nil").unwrap();
        assert!(is_nil);
    }

    // -- Error cases --

    #[test]
    fn build_error_surfaces_in_lua() {
        let e = engine();
        let result = e.exec(
            r#"
            bpc.layout("bad")
                :field("", 8)
                :build()
        "#,
        );
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("field name must not be empty"),
            "got: {err_msg}"
        );
    }

    #[test]
    fn resolve_error_surfaces_in_lua() {
        let e = engine();
        e.exec(
            r#"
            template = bpc.layout("resolve_err")
                :field("len", 8)
                :field_var("data", "len", "bytes")
                :build()
        "#,
        )
        .unwrap();

        // Empty data → InsufficientData
        let result = e.exec("template:resolve(bpc.bytes())");
        assert!(result.is_err());
    }

    #[test]
    fn double_build_errors() {
        let e = engine();
        let result = e.exec(
            r#"
            local b = bpc.layout("double"):field("x", 8)
            b:build()
            b:build()
        "#,
        );
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("already consumed"), "got: {err_msg}");
    }

    #[test]
    fn require_bpc_works() {
        let e = engine();
        let result: i64 = e
            .eval(
                r#"
            local bpc = require("bpc")
            local l = bpc.layout("req_test"):field("x", 8):build()
            return l:bit_len()
        "#,
            )
            .unwrap();
        assert_eq!(result, 8);
    }

    // -- field_at explicit positioning --

    #[test]
    fn layout_field_at_explicit() {
        let e = engine();
        e.exec(
            r#"
            layout = bpc.layout("explicit")
                :field_at("flags", 24, 8)
                :field_at("version", 0, 3)
                :build()
        "#,
        )
        .unwrap();

        let bit_len: i64 = e.eval("return layout:bit_len()").unwrap();
        assert_eq!(bit_len, 32);

        let ver_off: i64 =
            e.eval("return layout:field('version'):get_offset()").unwrap();
        assert_eq!(ver_off, 0);
    }

    // -- Full protocol example --

    #[test]
    fn full_protocol_example() {
        let e = engine();
        e.exec(
            r#"
            local bpc = require("bpc")

            -- Define a TLV-like protocol
            local proto = bpc.layout("tlv")
                :field("tag", 8)
                :field("length", 8)
                :field_var("value", "length", "bytes")
                :build()

            -- Parse a packet: tag=0x01, length=0x04, value=4 bytes
            local packet = bpc.hex("01 04 DE AD BE EF")
            local parsed = proto:resolve(packet)

            -- Verify structure
            assert(parsed:bit_len() == 48, "expected 48 bits")
            assert(parsed:field_count() == 3, "expected 3 fields")

            -- Read values
            assert(parsed:read_field("tag", packet) == 1, "tag should be 1")
            assert(parsed:read_field("length", packet) == 4, "length should be 4")

            -- Verify field positions
            local vf = parsed:field("value")
            assert(vf:get_offset() == 16, "value offset should be 16")
            assert(vf:get_width() == 32, "value width should be 32")
        "#,
        )
        .unwrap();
    }
}
