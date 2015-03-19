//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

/// Error type for codec operations.
// TODO: Perhaps we should have separate error types for codec and byte_vector
#[derive(Debug)]
pub struct Error {
    /// The error message.
    pub description: String
}
