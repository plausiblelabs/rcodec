//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

/// An immutable vector of bytes.
pub trait ByteVector {

    /// Return the length, in bytes.
    fn length(&self) -> u64;
}

/// ByteVector implementation.
// TODO: Only public because of type declaration issue below
pub struct ByteVectorImpl {
    /// The underlying storage type.
    storage: StorageType
}

impl ByteVector for ByteVectorImpl {

    // From ByteVector trait
    fn length(&self)-> u64 {
        match self.storage {
            StorageType::Empty => 0,
//            StorageType::DirectValue => 4,
        }
    }
}

/// A sum type over all supported storage object types.
enum StorageType  {
    Empty,
//    DirectValue// { v: Int }
}

/// An empty byte vector.
// TODO: Is it possible to declare this as the trait type?
pub static EMPTY: ByteVectorImpl = ByteVectorImpl { storage: StorageType::Empty };

///// Byte vector that directly stores a certain number of bytes.
//pub fn direct<T>(v: T) -> ByteVector {
//    EMPTY;
//}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_of_empty_vector_should_be_zero() {
        assert_eq!(EMPTY.length(), 0);
    }

//    #[test]
//    fn length_of_direct_vector_should_be_size_of_integer() {
//        assert_eq!(direct(7u8).length(), 1);
//    }
}
