//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

// A note from:
//   http://blog.burntsushi.net/rust-regex-syntax-extensions
//
// This attribute specifies that the Rust compiler will output a dynamic 
// library when this file is compiled.
//
// Generally, many Rust libraries will *also* have a `#![crate_type = "rlib"]`
// attribute set, which means the Rust compiler will produce a static library.
// However, libraries which provide syntax extensions must be dynamically 
// linked with `libsyntax`, so we elide the `rlib` and only produce a dynamic 
// library.
#![crate_type = "dylib"]
#![feature(rustc_private, plugin_registrar, quote)]

extern crate syntax;
extern crate rustc;

use syntax::ast;
use syntax::codemap;
use syntax::ext::base::{ExtCtxt, MacResult, MacEager};
use rustc::plugin::Registry;
 
#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("record_struct", expand_record_struct)
}

fn expand_record_struct(cx: &mut ExtCtxt, _: codemap::Span, _: &[ast::TokenTree]) -> Box<MacResult> {
    let answer = 5u64;
    MacEager::expr(quote_expr!(cx, $answer))
}
