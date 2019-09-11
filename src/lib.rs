//
// Copyright (c) 2015-2019 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

// TODO: Restore benchmark support
// // The following allows for benchmark tests.
// #![feature(test)]

// The following is necessary to make exported macros visible.
#[macro_use]
pub mod macros;

pub mod error;
pub mod byte_vector;
pub mod codec;

// TODO: Restore benchmark support
// // The following is used for benchmark tests.
// extern crate test;
