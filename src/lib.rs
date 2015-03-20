//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

// The following suppresses warnings related to use of unstable std::slice::bytes::copy_memory() function.
#![feature(core)]

pub mod error;
pub mod byte_vector;
pub mod codec;
