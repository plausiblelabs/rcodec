//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

// The following suppresses warnings related to use of unstable std::slice::bytes::copy_memory() function.
#![feature(core)]

// The following allows for benchmark tests.
#![feature(test)]

// The following allows for macro debugging via trace_macros(true/false).
#![feature(trace_macros)]

// The following allows for using macros defined in the separate rcodec_macros crate.
#![feature(plugin, custom_attribute)]
#![plugin(rcodec_macros)]

// The following allows for use of the unstable `slice_bytes` functionality.
#![feature(slice_bytes)]

// The following is necessary to make exported macros visible.
#[macro_use]
pub mod macros;

pub mod error;
pub mod hlist;
pub mod byte_vector;
pub mod codec;

// The following allows us to make use of the core crate (e.g. for core::ops).
extern crate core;

// The following is used for integral codecs.
extern crate num;

// The following is used for benchmark tests.
extern crate test;

