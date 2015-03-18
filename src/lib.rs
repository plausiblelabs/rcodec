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
#[derive(Debug)]
pub struct ByteVector {
    /// The underlying storage type.
    storage: Rc<StorageType>
}

impl ByteVector {
    /// Return the length, in bytes.
    pub fn length(&self) -> usize {
        self.storage.length()
    }

    /// Read up to a maximum of length bytes at offset from this byte vector into the given buffer.
    pub fn read(&self, buf: &mut [u8], offset: usize, len: usize) -> Result<usize, Error> {
        self.storage.read(buf, offset, len)
    }
}

/// Error type for byte vector operations.
#[derive(Debug)]
pub struct Error {
    /// The error message.
    pub description: String
}

impl PartialEq for ByteVector {
    fn eq(&self, other: &ByteVector) -> bool {
        if self.length() != other.length() {
            return false;
        }

        // This is a pretty inefficient implementation that reads a single byte at a time
        let len = self.length() as usize;
        for i in 0..len {
            let lhs = self.storage.unsafe_get(i);
            let rhs = other.storage.unsafe_get(i);
            if lhs != rhs {
                return false;
            }
        }

        true
    }
}

/// A sum type over all supported storage object types.
#[derive(Debug)]
enum StorageType {
    Empty,
    Heap { bytes: Vec<u8> },
    Append { lhs: Rc<StorageType>, rhs: Rc<StorageType>, len: usize }
}

impl StorageType {
    /// Return the length, in bytes.
    fn length(&self) -> usize {
        match *self {
            StorageType::Empty => 0,
            StorageType::Heap { ref bytes } => bytes.len(),
            StorageType::Append { ref len, .. } => *len
        }
    }

    /// Read up to a maximum of length bytes at offset from this byte vector into the given buffer.
    fn read(&self, buf: &mut [u8], offset: usize, len: usize) -> Result<usize, Error> {
        // Verify that range is within our storage bounds
        if offset + len > self.length() {
            return Err(Error { description: format!("Requested read offset of {off} bytes and length {len} exceeds vector length of {vlen}", off = offset, len = len, vlen = self.length()) });
        }

        match *self {
            StorageType::Empty => {
                Ok(0)
            },
            StorageType::Heap { ref bytes } => {
                let count = std::cmp::min(len, bytes.len() - offset);
                std::slice::bytes::copy_memory(buf, &bytes[offset .. offset + count]);
                Ok(count)
            },
            StorageType::Append { ref lhs, ref rhs, .. } => {
                // If the offset falls within lhs, perform the first half of the read
                let lhs_result = if offset < lhs.length() {
                    let lcount = std::cmp::min(lhs.length() - offset, len);
                    lhs.read(buf, offset, lcount)
                } else {
                    Ok(0)
                };

                // Then perform the rhs half of the read, if needed
                match lhs_result {
                    Ok(lhs_read_size) => {
                        let rhs_result = if lhs_read_size < len {
                            // Calculate the remaining offset
                            let roff = if lhs.length() < offset {
                                offset - lhs.length()
                            } else {
                                0
                            };
                            let rcount = len - lhs_read_size;
                            let dst = &mut buf[lhs_read_size .. lhs_read_size + rcount];
                            rhs.read(dst, roff, rcount) 
                        } else {
                            Ok(0)
                        };

                        rhs_result.map(|rhs_read_size| {
                            lhs_read_size + rhs_read_size
                        })
                    },
                    Err(e) => Err(e)
                }
            }
        }
    }

    /// Unsafe access by index.
    fn unsafe_get(&self, index: usize) -> u8 {
        let v: &mut[u8] = &mut[0];

        // Panic if the read failed
        let bytes_read = self.read(v, index, 1).unwrap();

        // Panic if we didn't read exactly one byte
        if bytes_read != 1 {
            panic!("Failed to read single byte");
        }

        // Otherwise, return the read value
        v[0]
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

    #[test]
    fn append_should_work() {
        let bytes = vec![1, 2, 3, 4];
        let lhs = buffered(&bytes);
        let rhs = buffered(&bytes);

        let bv = append(&lhs, &rhs);
        assert_eq!(bv.length(), 8);

        let expected = buffered(&vec![1, 2, 3, 4, 1, 2, 3, 4]);
        assert_eq!(bv, expected);
    }

    #[test]
    fn read_should_fail_if_offset_is_out_of_bounds() {
        let bytes = vec![1, 2, 3, 4];
        let bv = buffered(&bytes);

        let buf: &mut[u8] = &mut[0, 0];
        assert!(bv.read(buf, 0, 2).is_ok());
        assert!(bv.read(buf, 2, 2).is_ok());
        assert!(bv.read(buf, 4, 1).is_err());
    }

    #[test]
    fn read_should_work_for_buffered_vector() {
        let bytes = vec![1, 2, 3, 4];
        let bv = buffered(&bytes);

        let buf: &mut[u8] = &mut[0, 0];
        let result = bv.read(buf, 1, 2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
        assert_eq!(buf, [2, 3]);
    }

    #[test]
    fn read_should_work_for_append_vector() {
        let bytes = vec![1, 2, 3, 4];
        let lhs = buffered(&bytes);
        let rhs = buffered(&bytes);
        let bv = append(&lhs, &rhs);

        let buf: &mut[u8] = &mut[0, 0];

        // Verify case where read takes from lhs only
        {
            let result = bv.read(buf, 0, 2);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 2);
            assert_eq!(buf, [1, 2]);
        }

        // Verify case where read takes from rhs only
        {
            let result = bv.read(buf, 5, 2);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 2);
            assert_eq!(buf, [2, 3]);
        }

        // Verify case where read takes from both lhs and rhs
        {
            let result = bv.read(buf, 3, 2);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 2);
            assert_eq!(buf, [4, 1]);
        }
    }
}
