//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

// The following allows for using custom HListSupport attribute defined in hlist_macros crate.
#![feature(plugin, custom_attribute)]
#![plugin(hlist_macros)]

// The following allows for benchmark tests.
#![feature(test)]

// The following allows for macro debugging via trace_macros(true/false).
#![feature(trace_macros)]

// The following is necessary to make exported macros visible.
#[macro_use]
pub mod macros;

pub mod error;
pub mod byte_vector;
pub mod codec;

#[macro_use]
extern crate hlist;

// The following allows us to make use of the core crate (e.g. for core::ops).
extern crate core;

// The following is used for integral codecs.
extern crate num;

// The following is used for benchmark tests.
extern crate test;
