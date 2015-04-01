//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

// The following suppresses warnings related to use of unstable std::slice::bytes::copy_memory() function.
#![feature(core)]

// The following allows for macro debugging via trace_macros(true/false).
#![feature(trace_macros)]

// The following allows for using macros defined in the separate rcodec_macros crate.
#![feature(plugin)]
#![plugin(rcodec_macros)]

// The following suppresses warnings related to unstable stuff used for file-backed ByteVectors.
#![feature(file_path)]
#![feature(io)]
#![feature(path_ext)]

// The following is necessary to make exported macros visible.
#[macro_use]
pub mod macros;

pub mod error;
pub mod hlist;
pub mod byte_vector;

// Let us have lowercase names for static codec instances, e.g. uint8, please.
#[allow(non_upper_case_globals)]
pub mod codec;

// The following allows us to make use of the core crate (e.g. for core::ops).
extern crate core;
