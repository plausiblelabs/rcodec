//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

/// An immutable vector of bytes.
pub struct ByteVector {
    /// The underlying storage type.
    storage: StorageType
}

impl ByteVector {
    /// Return the length, in bytes.
    fn length(&self)-> u64 {
        match self.storage {
            StorageType::Empty => 0,
            StorageType::Heap { ref bytes } => bytes.len() as u64,
//            StorageType::Direct { len, bytes } => len
        }
    }
}

/// A sum type over all supported storage object types.
enum StorageType  {
    Empty,
    Heap { bytes: std::vec::Vec<u8> },
//    Direct { len: u64, bytes: [u8, ..8] }
}

/// An empty byte vector.
pub static EMPTY: ByteVector = ByteVector { storage: StorageType::Empty };

/// Return a byte vector that stores a certain number of bytes on the heap.
pub fn buffered(bytes: Vec<u8>) -> ByteVector {
    let storage = StorageType::Heap { bytes: bytes };
    ByteVector { storage: storage }
}

/// Return a byte vector that directly stores a certain number of bytes on the stack.
//pub fn direct<T>(v: T) -> ByteVector {
//    let storage = StorageType::Direct { len: 1, bytes: [0u8, ..8] };
//    ByteVectorImpl { storage: storage }
//}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_of_empty_vector_should_be_zero() {
        assert_eq!(EMPTY.length(), 0);
    }

    #[test]
    fn length_of_buffered_vector_should_be_correct() {
        let bytes = vec![1, 2, 3, 4];
        assert_eq!(buffered(bytes).length(), 4);
    }

    //    #[test]
//    fn length_of_direct_vector_should_be_size_of_integer() {
//        assert_eq!(direct(7u8).length(), 1);
//    }
}
