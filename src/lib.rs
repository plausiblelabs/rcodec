//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

// The following suppresses warnings related to use of unstable std::slice::bytes::copy_memory() function.
#![feature(core)]

// The following is necessary to make exported macros visible.
#[macro_use]
pub mod macros;

pub mod error;
pub mod hlist;
pub mod byte_vector;
pub mod codec;

// The following allows us to make use of the core crate (e.g. for core::ops).
extern crate core;
