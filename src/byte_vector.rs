//
// Copyright (c) 2015-2019 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

use core::fmt;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::rc::Rc;
use std::vec::Vec;

use crate::error::Error;

/// An immutable vector of bytes.
#[derive(Clone)]
pub struct ByteVector {
    /// The underlying storage type.
    storage: Rc<StorageType>,
}

impl ByteVector {
    /// Returns the length, in bytes.
    pub fn length(&self) -> usize {
        self.storage.length()
    }

    /// Reads up to a maximum of `len` bytes at `offset` from this byte vector into the given buffer.
    pub fn read(&self, buf: &mut [u8], offset: usize, len: usize) -> Result<usize, Error> {
        self.storage.read(buf, offset, len)
    }

    /// Converts this byte vector to a `Vec<u8>` instance. Note that this will copy all of the underlying
    /// data, so beware the increased memory usage.
    pub fn to_vec(&self) -> Result<Vec<u8>, Error> {
        // Allocate a buffer large enough to hold the backing bytes
        let mut vec = vec![0u8; self.length()];

        // Read from the byte vector into our mutable buffer, then return the buffer if successful
        // TODO: Check that all bytes were read?
        self.read(&mut vec[..], 0, self.length()).map(|_res| vec)
    }

    /// Returns a new byte vector containing exactly `len` bytes from this byte vector, or an
    /// error if insufficient data is available.
    pub fn take(&self, len: usize) -> Result<ByteVector, Error> {
        ByteVector::view(&self.storage, 0, len).map(|storage| ByteVector { storage })
    }

    /// Returns a new byte vector containing all but the first `len` bytes of this byte vector,
    /// or an error if dropping `len` bytes would overrun the end of this byte vector.
    pub fn drop(&self, len: usize) -> Result<ByteVector, Error> {
        let storage_len = self.length();
        if len > storage_len {
            return Err(Error::new(format!(
                "Requested length of {len} bytes exceeds vector length of {vlen}",
                len = len,
                vlen = storage_len
            )));
        }

        ByteVector::view(&self.storage, len, storage_len - len)
            .map(|remainder| ByteVector { storage: remainder })
    }

    /// Returns a new vector of length `len` containing zero or more low bytes followed by this byte vector's contents.
    /// If this vector is longer than `len` bytes, an error will be returned.
    pub fn pad_left(&self, len: usize) -> Result<ByteVector, Error> {
        #![allow(clippy::unknown_clippy_lints, clippy::comparison_chain)]

        let storage_len = self.length();
        if len < storage_len {
            Err(Error::new(format!(
                "Requested padded length of {len} bytes is smaller than vector length of {vlen}",
                len = len,
                vlen = storage_len
            )))
        } else if len == storage_len {
            Ok((*self).clone())
        } else {
            Ok(append(&fill(0, len - storage_len), self))
        }
    }

    /// Returns a new vector of length `len` containing this byte vector's contents followed by zero or more low bytes.
    /// If this vector is longer than `len` bytes, an error will be returned.
    pub fn pad_right(&self, len: usize) -> Result<ByteVector, Error> {
        #![allow(clippy::unknown_clippy_lints, clippy::comparison_chain)]

        let storage_len = self.length();
        if len < storage_len {
            Err(Error::new(format!(
                "Requested padded length of {len} bytes is smaller than vector length of {vlen}",
                len = len,
                vlen = storage_len
            )))
        } else if len == storage_len {
            Ok((*self).clone())
        } else {
            Ok(append(self, &fill(0, len - storage_len)))
        }
    }

    /// Returns a projection at `offset` with `len` bytes within the given storage.
    fn view(
        storage: &Rc<StorageType>,
        offset: usize,
        len: usize,
    ) -> Result<Rc<StorageType>, Error> {
        // Verify that offset is within our storage bounds
        let storage_len = storage.length();
        if offset > storage_len {
            return Err(Error::new(format!(
                "Requested view offset of {off} bytes exceeds vector length of {vlen}",
                off = offset,
                vlen = storage_len
            )));
        }

        // Verify that offset + len will not overflow
        if std::usize::MAX - offset < len {
            return Err(Error::new(format!("Requested view offset of {off} and length {len} bytes would overflow maximum value of usize", off = offset, len = len)));
        }

        // Verify that offset + len is within our storage bounds
        if offset + len > storage_len {
            return Err(Error::new(format!("Requested view offset of {off} and length {len} bytes exceeds vector length of {vlen}", off = offset, len = len, vlen = storage_len)));
        }

        // Return storage unmodified if the requested length equals the storage length
        if len == storage_len {
            return Ok((*storage).clone());
        }

        match **storage {
            StorageType::Empty => Err(Error::new(
                "Cannot create view for empty vector".to_string(),
            )),

            StorageType::DirectValue { .. } => {
                // Create a new view around the value storage
                Ok(Rc::new(StorageType::View {
                    vstorage: (*storage).clone(),
                    voffset: offset,
                    vlen: len,
                }))
            }

            StorageType::Heap { .. } => {
                // Create a new view around this heap storage
                Ok(Rc::new(StorageType::View {
                    vstorage: (*storage).clone(),
                    voffset: offset,
                    vlen: len,
                }))
            }

            StorageType::Append {
                ref lhs, ref rhs, ..
            } => {
                // If a single side encompasses the requested range, create a View around that side;
                // otherwise the range spans both sides and we need to construct a new Append with
                // two new Views
                let lhs_len = lhs.length();
                if offset + len < lhs_len {
                    // Drop the entire rhs
                    ByteVector::view(&lhs, offset, len)
                } else if offset >= lhs_len {
                    // Drop the entire lhs
                    let rhs_offset = offset - lhs_len;
                    ByteVector::view(&rhs, rhs_offset, len)
                } else {
                    // Create a new Append that spans portions of lhs and rhs
                    let lhs_view_len = lhs_len - offset;
                    let rhs_view_len = len - lhs_view_len;
                    forcomp!({
                        lhs_view <- ByteVector::view(&lhs, offset, lhs_view_len);
                        rhs_view <- ByteVector::view(&rhs, 0, rhs_view_len);
                    } yield {
                        Rc::new(StorageType::Append { lhs: lhs_view, rhs: rhs_view, len: lhs_view_len + rhs_view_len })
                    })
                }
            }

            StorageType::View {
                ref vstorage,
                ref voffset,
                ..
            } => {
                // Verify that voffset + offset will not overflow
                if std::usize::MAX - offset < *voffset {
                    return Err(Error::new(format!("Requested view offset of {off} plus storage offset {voff} would overflow maximum value of usize", off = offset, voff = *voffset)));
                }
                ByteVector::view(vstorage, *voffset + offset, len)
            }

            StorageType::File { .. } => {
                // Create a new view around the file storage
                Ok(Rc::new(StorageType::View {
                    vstorage: (*storage).clone(),
                    voffset: offset,
                    vlen: len,
                }))
            }
        }
    }
}

impl PartialEq for ByteVector {
    fn eq(&self, other: &ByteVector) -> bool {
        if self.length() != other.length() {
            return false;
        }

        // This is a pretty inefficient implementation that reads a single byte at a time
        let len = self.length();
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

impl Eq for ByteVector {}

const CHARS: &[u8] = b"0123456789abcdef";

impl Debug for ByteVector {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let len = self.length();
        let mut v = Vec::with_capacity(len * 2);
        for i in 0..len {
            let byte = self.storage.unsafe_get(i);
            v.push(CHARS[(byte >> 4) as usize]);
            v.push(CHARS[(byte & 0xf) as usize]);
        }
        unsafe {
            let result = f.write_str(&String::from_utf8_unchecked(v));
            if result.is_err() {
                return result;
            }
        };
        Ok(())
    }
}

// Wrapper around File that provides an implementation of Debug
struct WrappedFile {
    file: RefCell<File>,
    path: String,
}

impl Debug for WrappedFile {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        formatter.write_str(&self.path)
    }
}

/// The maximum size that can be used with a `DirectValue` storage type.
#[doc(hidden)]
pub const DIRECT_VALUE_SIZE_LIMIT: usize = 8;

/// A sum type over all supported storage object types.
#[derive(Debug)]
enum StorageType {
    Empty,
    DirectValue {
        bytes: [u8; DIRECT_VALUE_SIZE_LIMIT],
        length: usize,
    },
    Heap {
        bytes: Vec<u8>,
    },
    Append {
        lhs: Rc<StorageType>,
        rhs: Rc<StorageType>,
        len: usize,
    },
    // TODO: Note the 'v' prefix; I couldn't find a way to rename the variables while destructuring
    // in a match, so this was the only way to avoid colliding with the offset/len function parameters
    View {
        vstorage: Rc<StorageType>,
        voffset: usize,
        vlen: usize,
    },
    File {
        file: WrappedFile,
        length: usize,
    },
}

impl StorageType {
    /// Returns the length, in bytes.
    fn length(&self) -> usize {
        match *self {
            StorageType::Empty => 0,
            StorageType::DirectValue { ref length, .. } => *length,
            StorageType::Heap { ref bytes } => bytes.len(),
            StorageType::Append { ref len, .. } => *len,
            StorageType::View { ref vlen, .. } => *vlen,
            StorageType::File { ref length, .. } => *length,
        }
    }

    /// Reads up to a maximum of length bytes at offset from this byte vector into the given buffer.
    fn read(&self, buf: &mut [u8], offset: usize, len: usize) -> Result<usize, Error> {
        // Verify that offset is within our storage bounds
        let storage_len = self.length();
        if offset > storage_len {
            return Err(Error::new(format!(
                "Requested read offset of {off} bytes exceeds vector length of {vlen}",
                off = offset,
                vlen = storage_len
            )));
        }

        // Verify that offset + len will not overflow
        if std::usize::MAX - offset < len {
            return Err(Error::new(format!("Requested read offset of {off} and length {len} bytes would overflow maximum value of usize", off = offset, len = len)));
        }

        // Verify that offset + len is within our storage bounds
        if offset + len > storage_len {
            return Err(Error::new(format!("Requested read offset of {off} and length {len} bytes exceeds vector length of {vlen}", off = offset, len = len, vlen = storage_len)));
        }

        match *self {
            StorageType::Empty => Err(Error::new("Cannot read from empty vector".to_string())),

            StorageType::DirectValue {
                ref bytes,
                ref length,
            } => {
                let count = std::cmp::min(len, *length - offset);
                copy_memory(&bytes[offset..offset + count], buf);
                Ok(count)
            }

            StorageType::Heap { ref bytes } => {
                let count = std::cmp::min(len, bytes.len() - offset);
                copy_memory(&bytes[offset..offset + count], buf);
                Ok(count)
            }

            StorageType::Append {
                ref lhs, ref rhs, ..
            } => {
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
                            let dst = &mut buf[lhs_read_size..lhs_read_size + rcount];
                            rhs.read(dst, roff, rcount)
                        } else {
                            Ok(0)
                        };

                        rhs_result.map(|rhs_read_size| lhs_read_size + rhs_read_size)
                    }
                    Err(e) => Err(e),
                }
            }

            StorageType::View {
                ref vstorage,
                ref voffset,
                ref vlen,
            } => {
                // Verify that voffset + offset won't overflow
                if std::usize::MAX - offset < *voffset {
                    return Err(Error::new(format!("Requested read offset of {off} plus storage offset {voff} would overflow maximum value of usize", off = offset, voff = *voffset)));
                }

                // Let the backing storage perform the read
                let count = std::cmp::min(*vlen, len);
                vstorage.read(buf, *voffset + offset, count)
            }

            StorageType::File {
                ref file,
                ref length,
            } => {
                let count = std::cmp::min(*length, len);
                let f = &mut file.file.borrow_mut();

                // Seek to `offset` and then read `count` bytes
                let read_result = f
                    .seek(SeekFrom::Start(offset as u64))
                    .and_then(|_newpos| f.read(&mut buf[0..count]))
                    .map_err(|io_err| Error::new(format!("Failed to read file: {}", io_err)));

                // If the read was incomplete, keep reading recursively
                read_result.and_then(|bytes_read| {
                    if bytes_read < count {
                        self.read(
                            &mut buf[bytes_read..len - bytes_read],
                            offset + bytes_read,
                            len - bytes_read,
                        )
                        .map(|size| size + bytes_read)
                    } else {
                        Ok(bytes_read)
                    }
                })
            }
        }
    }

    /// Unsafe access by index.
    fn unsafe_get(&self, index: usize) -> u8 {
        let v: &mut [u8] = &mut [0];

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

/// Returns an empty byte vector.
// TODO: Statics can't refer to heap-allocated data, so we can't have a single instance here
//pub static EMPTY: ByteVector = ByteVector { storage: Rc::new(StorageType::Empty) };
pub fn empty() -> ByteVector {
    ByteVector {
        storage: Rc::new(StorageType::Empty),
    }
}

/// Returns a byte vector that consumes the contents of the given `Vec<u8>`.
pub fn from_vec(bytes: Vec<u8>) -> ByteVector {
    let storage = StorageType::Heap { bytes };
    ByteVector {
        storage: Rc::new(storage),
    }
}

/// Returns a byte vector that stores a copy of the given bytes on the heap.
pub fn from_slice_copy(bytes: &[u8]) -> ByteVector {
    let storage = if bytes.len() <= DIRECT_VALUE_SIZE_LIMIT {
        let mut array = [0u8; DIRECT_VALUE_SIZE_LIMIT];
        copy_memory(bytes, &mut array);
        StorageType::DirectValue {
            bytes: array,
            length: bytes.len(),
        }
    } else {
        StorageType::Heap {
            bytes: bytes.to_owned(),
        }
    };
    ByteVector {
        storage: Rc::new(storage),
    }
}

/// Returns a byte vector that consumes the given slice, used to store primitive values directly.
pub fn from_slice(bytes: [u8; DIRECT_VALUE_SIZE_LIMIT], length: usize) -> ByteVector {
    ByteVector {
        storage: Rc::new(StorageType::DirectValue { bytes, length }),
    }
}

/// Returns a byte vector whose contents come from a file.
pub fn file(path: &Path) -> Result<ByteVector, Error> {
    // Open the file at the given path and create a ByteVector around it
    let result = forcomp!({
        file <- File::open(path);
        metadata <- path.metadata();
    } yield {
        ByteVector {
            storage: Rc::new(StorageType::File {
                file: WrappedFile {
                    file: RefCell::new(file),
                    path: format!("{}", path.display())
                },
                length: metadata.len() as usize
            })
        }
    });

    // Wrap I/O error in an rcodec error, if needed
    result.map_err(|io_err| Error::new(format!("Failed to open file: {}", io_err)))
}

/// Returns a byte vector that contains the contents of `lhs` followed by the contents of `rhs`.
pub fn append(lhs: &ByteVector, rhs: &ByteVector) -> ByteVector {
    if lhs.length() == 0 && rhs.length() == 0 {
        empty()
    } else if lhs.length() == 0 {
        ByteVector {
            storage: rhs.storage.clone(),
        }
    } else if rhs.length() == 0 {
        ByteVector {
            storage: lhs.storage.clone(),
        }
    } else {
        let storage = StorageType::Append {
            lhs: lhs.storage.clone(),
            rhs: rhs.storage.clone(),
            len: lhs.storage.length() + rhs.storage.length(),
        };
        ByteVector {
            storage: Rc::new(storage),
        }
    }
}

/// Returns a byte vector containing `value` repeated `count` times.
pub fn fill(value: u8, count: usize) -> ByteVector {
    let storage = StorageType::Heap {
        bytes: vec![value; count],
    };
    ByteVector {
        storage: Rc::new(storage),
    }
}

/// A replacement for the deprecated std::slice::bytes::copy_memory
fn copy_memory(from: &[u8], mut to: &mut [u8]) -> usize {
    use std::io::Write;
    to.write(from).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn byte_vector_macro_should_work() {
        let bv1 = from_vec(vec![1, 2, 3, 4]);
        let bv2 = byte_vector!(1, 2, 3, 4);
        assert_eq!(bv1, bv2);
    }

    #[test]
    fn clone_should_work() {
        let bytes = vec![1, 2, 3, 4];
        let lhs = from_slice_copy(&bytes);
        let rhs = from_slice_copy(&bytes);
        let bv1 = append(&lhs, &rhs);
        let bv2 = bv1.clone();
        assert_eq!(bv1, bv2);
    }

    #[test]
    fn debug_string_should_be_formatted_correctly() {
        assert_eq!("01020eff", format!("{:?}", byte_vector!(1, 2, 14, 255)))
    }

    #[test]
    fn length_of_empty_vector_should_be_zero() {
        assert_eq!(empty().length(), 0);
    }

    #[test]
    fn length_of_heap_vector_should_be_correct() {
        assert_eq!(byte_vector!(1, 2, 3, 4).length(), 4);
    }

    #[test]
    fn append_should_work() {
        let bytes = vec![1, 2, 3, 4];
        let lhs = from_slice_copy(&bytes);
        let rhs = from_slice_copy(&bytes);

        let bv = append(&lhs, &rhs);
        assert_eq!(bv.length(), 8);

        let expected = byte_vector!(1, 2, 3, 4, 1, 2, 3, 4);
        assert_eq!(bv, expected);
    }

    #[test]
    fn big_appends_should_work() {
        let small = from_vec(vec![1; DIRECT_VALUE_SIZE_LIMIT]);
        let big = from_vec(vec![2; DIRECT_VALUE_SIZE_LIMIT + 1]);

        let smallbig = append(&small, &big);
        let mut smallbig_expected = vec![1; DIRECT_VALUE_SIZE_LIMIT];
        smallbig_expected.extend(vec![2; DIRECT_VALUE_SIZE_LIMIT + 1]);
        assert_eq!(smallbig, from_vec(smallbig_expected));

        let bigsmall = append(&big, &small);
        let mut bigsmall_expected = vec![2; DIRECT_VALUE_SIZE_LIMIT + 1];
        bigsmall_expected.extend(vec![1; DIRECT_VALUE_SIZE_LIMIT]);
        assert_eq!(bigsmall, from_vec(bigsmall_expected));

        let bigbig = append(&big, &big);
        let bigbig_expected = vec![2; DIRECT_VALUE_SIZE_LIMIT * 2 + 2];
        assert_eq!(bigbig, from_vec(bigbig_expected));
    }

    #[test]
    fn fill_should_work() {
        let bv = fill(6u8, 4);
        let expected = byte_vector!(6, 6, 6, 6);
        assert_eq!(bv, expected);
    }

    #[test]
    fn read_should_fail_if_offset_is_out_of_bounds() {
        let bv = byte_vector!(1, 2, 3, 4);

        let buf: &mut [u8] = &mut [0, 0];
        assert!(bv.read(buf, 0, 2).is_ok());
        assert!(bv.read(buf, 2, 2).is_ok());
        assert!(bv.read(buf, 4, 1).is_err());

        // TODO: Also test overflow case
    }

    #[test]
    fn read_should_work_for_heap_vector() {
        let bv = byte_vector!(1, 2, 3, 4);

        let buf: &mut [u8] = &mut [0, 0];
        let result = bv.read(buf, 1, 2);
        assert_eq!(result.unwrap(), 2);
        assert_eq!(buf, [2, 3]);
    }

    #[test]
    fn read_should_work_for_append_vector() {
        let bytes = vec![1, 2, 3, 4];
        let lhs = from_slice_copy(&bytes);
        let rhs = from_slice_copy(&bytes);
        let bv = append(&lhs, &rhs);

        let buf: &mut [u8] = &mut [0, 0];

        // Verify case where read takes from lhs only
        {
            let result = bv.read(buf, 0, 2);
            assert_eq!(result.unwrap(), 2);
            assert_eq!(buf, [1, 2]);
        }

        // Verify case where read takes from rhs only
        {
            let result = bv.read(buf, 5, 2);
            assert_eq!(result.unwrap(), 2);
            assert_eq!(buf, [2, 3]);
        }

        // Verify case where read takes from both lhs and rhs
        {
            let result = bv.read(buf, 3, 2);
            assert_eq!(result.unwrap(), 2);
            assert_eq!(buf, [4, 1]);
        }
    }

    #[test]
    fn read_should_work_for_nested_views() {
        let bv = byte_vector!(1, 2, 3, 4);
        let view0 = bv.drop(1).unwrap();
        let view1 = view0.drop(1).unwrap();

        let buf: &mut [u8] = &mut [0, 0];
        assert_eq!(view1.read(buf, 0, 2).unwrap(), 2);
        assert_eq!(buf, [3, 4]);

        // TODO: Also test overflow case
    }

    #[test]
    fn to_vec_should_work() {
        let input = vec![1, 2, 3, 4];
        let lhs = from_slice_copy(&input);
        let rhs = from_slice_copy(&input);
        let bv = append(&lhs, &rhs);

        let result = bv.to_vec();
        assert_eq!(result.unwrap(), vec!(1, 2, 3, 4, 1, 2, 3, 4));
    }

    #[test]
    fn take_should_fail_if_length_is_invalid() {
        let bv = byte_vector!(1, 2, 3, 4);

        assert!(bv.take(2).is_ok());
        assert!(bv.take(4).is_ok());
        assert!(bv.take(5).is_err());
    }

    #[test]
    fn take_should_work_for_heap_vector() {
        let bv = byte_vector!(1, 2, 3, 4);

        let result = bv.take(2);
        assert_eq!(result.unwrap(), byte_vector!(1, 2));
    }

    #[test]
    fn take_should_work_for_append_vector() {
        let bytes = vec![1, 2, 3, 4];
        let lhs = from_slice_copy(&bytes);
        let rhs = from_slice_copy(&bytes);
        let bv = append(&lhs, &rhs);

        // Verify case where take takes part of lhs only
        {
            let result = bv.take(2);
            assert_eq!(result.unwrap(), byte_vector!(1, 2));
        }

        // Verify case where take takes from both lhs and rhs
        {
            let result = bv.take(6);
            assert_eq!(result.unwrap(), byte_vector!(1, 2, 3, 4, 1, 2));
        }
    }

    #[test]
    fn drop_should_fail_if_length_is_invalid() {
        let bv = byte_vector!(1, 2, 3, 4);

        assert!(bv.drop(2).is_ok());
        assert!(bv.drop(4).is_ok());
        assert!(bv.drop(5).is_err());
    }

    #[test]
    fn drop_should_work_for_heap_vector() {
        let bv = byte_vector!(1, 2, 3, 4);

        let result = bv.drop(2);
        assert_eq!(result.unwrap(), byte_vector!(3, 4));
    }

    #[test]
    fn drop_should_work_for_append_vector() {
        let bytes = vec![1, 2, 3, 4];
        let lhs = from_slice_copy(&bytes);
        let rhs = from_slice_copy(&bytes);
        let bv = append(&lhs, &rhs);

        // Verify case where drop takes part of lhs only
        {
            let result = bv.drop(2);
            assert_eq!(result.unwrap(), byte_vector!(3, 4, 1, 2, 3, 4));
        }

        // Verify case where drop takes from both lhs and rhs
        {
            let result = bv.drop(6);
            assert_eq!(result.unwrap(), byte_vector!(3, 4));
        }
    }

    #[test]
    fn pad_left_should_work() {
        let bv = byte_vector!(1, 2, 3, 4);
        assert_eq!(bv.pad_left(4).unwrap(), byte_vector!(1, 2, 3, 4));
        assert_eq!(bv.pad_left(5).unwrap(), byte_vector!(0, 1, 2, 3, 4));
        assert_eq!(bv.pad_left(6).unwrap(), byte_vector!(0, 0, 1, 2, 3, 4));
    }

    #[test]
    fn pad_left_should_fail_if_length_is_invalid() {
        let bv = byte_vector!(1, 2, 3, 4);
        assert_eq!(
            bv.pad_left(3).unwrap_err().message(),
            "Requested padded length of 3 bytes is smaller than vector length of 4"
        );
    }

    #[test]
    fn pad_right_should_work() {
        let bv = byte_vector!(1, 2, 3, 4);
        assert_eq!(bv.pad_right(4).unwrap(), byte_vector!(1, 2, 3, 4));
        assert_eq!(bv.pad_right(5).unwrap(), byte_vector!(1, 2, 3, 4, 0));
        assert_eq!(bv.pad_right(6).unwrap(), byte_vector!(1, 2, 3, 4, 0, 0));
    }

    #[test]
    fn pad_right_should_fail_if_length_is_invalid() {
        let bv = byte_vector!(1, 2, 3, 4);
        assert_eq!(
            bv.pad_right(3).unwrap_err().message(),
            "Requested padded length of 3 bytes is smaller than vector length of 4"
        );
    }

    #[test]
    fn file_should_work() {
        use std::io::Write;
        use std::path::Path;
        let path = Path::new("/tmp/rcodec-test-file");

        let contents = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut write_file = match fs::File::create(path) {
            Err(why) => panic!("Couldn't create test file {:?}: {}", path.to_str(), why),
            Ok(file) => file,
        };
        if let Err(why) = write_file.write_all(&contents) {
            panic!("Couldn't write test file {:?}: {}", path.to_str(), why)
        }

        let bv_result = file(path);
        assert!(bv_result.is_ok());
        let bv = bv_result.unwrap();
        assert_eq!(bv, byte_vector!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10));

        let dropped = bv.drop(5);
        assert!(dropped.is_ok());
        assert_eq!(dropped.unwrap(), byte_vector!(6, 7, 8, 9, 10));

        let _ignore = fs::remove_file(&path);
    }
}
