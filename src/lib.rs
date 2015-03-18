//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

use std::rc::Rc;
use std::vec::Vec;

/// An immutable vector of bytes.
pub struct ByteVector {
    /// The underlying storage type.
    storage: Rc<StorageType>
}

impl ByteVector {
    /// Return the length, in bytes.
    pub fn length(&self)-> u64 {
        self.storage.length()
    }
}

/// A sum type over all supported storage object types.
enum StorageType {
    Empty,
    Heap { bytes: Vec<u8> },
//    Direct { len: u64, bytes: [u8, ..8] },
    Append { lhs: Rc<StorageType>, rhs: Rc<StorageType>, len: u64 }
}

impl StorageType {
    /// Return the length, in bytes.
    pub fn length(&self)-> u64 {
        match *self {
            StorageType::Empty => 0,
            StorageType::Heap { ref bytes } => bytes.len() as u64,
//            StorageType::Direct { len, bytes } => len,
            StorageType::Append { ref len, .. } => *len
        }
    }
}

/// An empty byte vector.
// TODO: Statics can't refer to heap-allocated data, so we can't have a single instance here
//pub static EMPTY: ByteVector = ByteVector { storage: Rc::new(StorageType::Empty) };
pub fn empty() -> ByteVector {
    ByteVector { storage: Rc::new(StorageType::Empty) }
}

/// Return a byte vector that stores a copy of the given bytes on the heap.
pub fn buffered(bytes: &Vec<u8>) -> ByteVector {
    // TODO: For now we only support copying, so that the returned ByteVector owns a copy
    let storage = StorageType::Heap { bytes: bytes.clone() };
    ByteVector { storage: Rc::new(storage) }
}

/// Return a byte vector that directly stores a certain number of bytes on the stack.
//pub fn direct<T>(v: T) -> ByteVector {
//    let storage = StorageType::Direct { len: 1, bytes: [0u8, ..8] };
//    ByteVector { storage: Rc::new(storage) }
//}

/// Return a byte vector that contains the contents of lhs followed by the contents of rhs.
pub fn append(lhs: &ByteVector, rhs: &ByteVector) -> ByteVector {
    let storage = StorageType::Append { lhs: lhs.storage.clone(), rhs: rhs.storage.clone(), len: lhs.storage.length() + rhs.storage.length() };
    ByteVector { storage: Rc::new(storage) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_of_empty_vector_should_be_zero() {
        assert_eq!(empty().length(), 0);
    }

    #[test]
    fn length_of_buffered_vector_should_be_correct() {
        let bytes = vec![1, 2, 3, 4];
        assert_eq!(buffered(&bytes).length(), 4);
    }

    //    #[test]
//    fn length_of_direct_vector_should_be_size_of_integer() {
//        assert_eq!(direct(7u8).length(), 1);
//    }

    #[test]
    fn append_should_work() {
        let bytes = vec![1, 2, 3, 4];
        let lhs = buffered(&bytes);
        let rhs = buffered(&bytes);

        let bv = append(&lhs, &rhs);
//        let expected = buffered(&vec![1, 2, 3, 4, 1, 2, 3, 4]);
        assert_eq!(bv.length(), 8);
//        assert_eq!(bv, expected);
    }
}
