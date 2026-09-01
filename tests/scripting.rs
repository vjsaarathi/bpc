//! Integration tests for the Lua scripting layer.

use bpc::scripting::ScriptEngine;

// -- Engine fundamentals --

#[test]
fn engine_creates_successfully() {
    ScriptEngine::new().unwrap();
}

#[test]
fn eval_arithmetic() {
    let e = ScriptEngine::new().unwrap();
    let r: i64 = e.eval("return 10 * 5 + 3").unwrap();
    assert_eq!(r, 53);
}

#[test]
fn exec_and_globals() {
    let e = ScriptEngine::new().unwrap();
    e.set_global("x", 10i64).unwrap();
    e.exec("y = x * 2").unwrap();
    let y: i64 = e.get_global("y").unwrap();
    assert_eq!(y, 20);
}

// -- bpc module availability --

#[test]
fn bpc_global_available() {
    let e = ScriptEngine::new().unwrap();
    let t: String = e.eval("return type(bpc)").unwrap();
    assert_eq!(t, "table");
}

#[test]
fn bpc_require_available() {
    let e = ScriptEngine::new().unwrap();
    let t: String = e.eval(r#"local b = require("bpc"); return type(b)"#).unwrap();
    assert_eq!(t, "table");
}

// -- Hex and bytes helpers --

#[test]
fn hex_with_spaces() {
    let e = ScriptEngine::new().unwrap();
    let b: Vec<u8> = e.eval(r#"return bpc.hex("DE AD BE EF")"#).unwrap();
    assert_eq!(b, vec![0xDE, 0xAD, 0xBE, 0xEF]);
}

#[test]
fn hex_lowercase() {
    let e = ScriptEngine::new().unwrap();
    let b: Vec<u8> = e.eval(r#"return bpc.hex("deadbeef")"#).unwrap();
    assert_eq!(b, vec![0xDE, 0xAD, 0xBE, 0xEF]);
}

#[test]
fn bytes_helper() {
    let e = ScriptEngine::new().unwrap();
    let b: Vec<u8> = e.eval("return bpc.bytes(0, 127, 255)").unwrap();
    assert_eq!(b, vec![0, 127, 255]);
}

// -- Layout declaration via Lua DSL --

#[test]
fn declare_simple_layout() {
    let e = ScriptEngine::new().unwrap();
    e.exec(
        r#"
        layout = bpc.layout("simple")
            :field("a", 8)
            :field("b", 16)
            :build()
    "#,
    )
    .unwrap();

    let bits: i64 = e.eval("return layout:bit_len()").unwrap();
    assert_eq!(bits, 24);

    let count: i64 = e.eval("return layout:field_count()").unwrap();
    assert_eq!(count, 2);
}

#[test]
fn declare_layout_with_variable_field() {
    let e = ScriptEngine::new().unwrap();
    e.exec(
        r#"
        proto = bpc.layout("var")
            :field("hdr", 4)
            :field("len", 4)
            :field_var("body", "len", "bytes")
            :build()
    "#,
    )
    .unwrap();

    let has_var: bool = e.eval("return proto:has_variable_fields()").unwrap();
    assert!(has_var);
}

// -- Resolve and read --

#[test]
fn resolve_and_read_fields() {
    let e = ScriptEngine::new().unwrap();
    e.exec(
        r#"
        local bpc = require("bpc")
        proto = bpc.layout("tlv")
            :field("tag", 8)
            :field("len", 8)
            :field_var("val", "len", "bytes")
            :build()

        data = bpc.hex("42 02 AA BB")
        resolved = proto:resolve(data)
    "#,
    )
    .unwrap();

    // Verify resolved structure.
    let bit_len: i64 = e.eval("return resolved:bit_len()").unwrap();
    assert_eq!(bit_len, 32);

    let val_w: i64 = e.eval("return resolved:field('val'):get_width()").unwrap();
    assert_eq!(val_w, 16);

    // Read field values.
    let tag: i64 = e.eval("return resolved:read_field('tag', data)").unwrap();
    assert_eq!(tag, 0x42);

    let len: i64 = e.eval("return resolved:read_field('len', data)").unwrap();
    assert_eq!(len, 2);
}

#[test]
fn resolve_same_template_different_data() {
    let e = ScriptEngine::new().unwrap();
    e.exec(
        r#"
        proto = bpc.layout("reuse")
            :field("len", 8)
            :field_var("data", "len", "bytes")
            :build()

        r1 = proto:resolve(bpc.bytes(1, 0xFF))
        r2 = proto:resolve(bpc.bytes(3, 0xAA, 0xBB, 0xCC))
    "#,
    )
    .unwrap();

    let r1_bits: i64 = e.eval("return r1:bit_len()").unwrap();
    assert_eq!(r1_bits, 16);

    let r2_bits: i64 = e.eval("return r2:bit_len()").unwrap();
    assert_eq!(r2_bits, 32);
}

// -- Field querying --

#[test]
fn field_by_index_lua_one_based() {
    let e = ScriptEngine::new().unwrap();
    e.exec(
        r#"
        layout = bpc.layout("idx")
            :field("x", 4)
            :field("y", 12)
            :build()
    "#,
    )
    .unwrap();

    let name1: String = e.eval("return layout:field(1):get_name()").unwrap();
    assert_eq!(name1, "x");
    let name2: String = e.eval("return layout:field(2):get_name()").unwrap();
    assert_eq!(name2, "y");
}

#[test]
fn field_at_bit_returns_nil_past_end() {
    let e = ScriptEngine::new().unwrap();
    e.exec("layout = bpc.layout('t'):field('x', 8):build()").unwrap();

    let is_nil: bool = e.eval("return layout:field_at_bit(8) == nil").unwrap();
    assert!(is_nil);
}

#[test]
fn fields_list_iteration() {
    let e = ScriptEngine::new().unwrap();
    e.exec(
        r#"
        layout = bpc.layout("iter")
            :field("a", 8)
            :field("b", 8)
            :field("c", 8)
            :build()

        names = {}
        for _, f in ipairs(layout:fields()) do
            table.insert(names, f:get_name())
        end
        result = table.concat(names, ",")
    "#,
    )
    .unwrap();

    let result: String = e.get_global("result").unwrap();
    assert_eq!(result, "a,b,c");
}

// -- Error handling --

#[test]
fn lua_build_error_is_catchable() {
    let e = ScriptEngine::new().unwrap();
    // Use pcall to catch the error in Lua.
    e.exec(
        r#"
        local ok, err = pcall(function()
            bpc.layout("bad"):field("x", 8):field("x", 8):build()
        end)
        caught = not ok
        errmsg = tostring(err)
    "#,
    )
    .unwrap();

    let caught: bool = e.get_global("caught").unwrap();
    assert!(caught);

    let errmsg: String = e.get_global("errmsg").unwrap();
    assert!(errmsg.contains("duplicate"), "got: {errmsg}");
}

#[test]
fn lua_resolve_error_is_catchable() {
    let e = ScriptEngine::new().unwrap();
    e.exec(
        r#"
        proto = bpc.layout("err")
            :field("len", 8)
            :field_var("data", "len", "bytes")
            :build()

        local ok, err = pcall(function()
            -- length=0 → zero width → error
            proto:resolve(bpc.bytes(0))
        end)
        caught = not ok
        errmsg = tostring(err)
    "#,
    )
    .unwrap();

    let caught: bool = e.get_global("caught").unwrap();
    assert!(caught);

    let errmsg: String = e.get_global("errmsg").unwrap();
    assert!(errmsg.contains("zero width"), "got: {errmsg}");
}

// -- End-to-end protocol parsing --

#[test]
fn end_to_end_dns_like_header() {
    let e = ScriptEngine::new().unwrap();
    e.exec(
        r#"
        local bpc = require("bpc")

        -- Simplified DNS-like header (12 bytes = 96 bits)
        local dns_header = bpc.layout("dns_header")
            :field("id", 16)
            :field("qr", 1)
            :field("opcode", 4)
            :field("aa", 1)
            :field("tc", 1)
            :field("rd", 1)
            :field("ra", 1)
            :field("z", 3)
            :field("rcode", 4)
            :field("qdcount", 16)
            :field("ancount", 16)
            :field("nscount", 16)
            :field("arcount", 16)
            :build()

        assert(dns_header:bit_len() == 96, "DNS header should be 96 bits")
        assert(dns_header:field_count() == 13, "DNS header should have 13 fields")
        assert(not dns_header:has_variable_fields(), "DNS header should be fixed")

        -- Verify specific field positions.
        local qr = dns_header:field("qr")
        assert(qr:get_offset() == 16, "QR should be at offset 16")
        assert(qr:get_width() == 1, "QR should be 1 bit")

        -- Read from actual data.
        -- ID=0x1234, flags=0x8180 (standard query response, recursion desired+available),
        -- qdcount=1, ancount=1, nscount=0, arcount=0.
        local data = bpc.hex("12 34 81 80 00 01 00 01 00 00 00 00")

        assert(dns_header:read_field("id", data) == 0x1234, "ID should be 0x1234")
        assert(dns_header:read_field("qr", data) == 1, "QR should be 1 (response)")
        assert(dns_header:read_field("rd", data) == 1, "RD should be 1")
        assert(dns_header:read_field("ra", data) == 1, "RA should be 1")
        assert(dns_header:read_field("qdcount", data) == 1, "QDCOUNT should be 1")
        assert(dns_header:read_field("ancount", data) == 1, "ANCOUNT should be 1")
    "#,
    )
    .unwrap();
}
