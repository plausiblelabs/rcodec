//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

use std::fmt::Display;
use std::mem::size_of;
use std::num;
use std::num::{Int, FromPrimitive, ToPrimitive};
use std::vec;
use core;

use error::Error;
use byte_vector;
use byte_vector::ByteVector;
use hlist::*;

/// Implements encoding and decoding of values of type `T`.
pub trait Codec<T> {
    /// Attempt to encode a value of type `T` into a ByteVector.
    fn encode(&self, value: &T) -> EncodeResult;
    
    /// Attempt to decode a value of type `T` from the given ByteVector.
    fn decode(&self, bv: &ByteVector) -> DecodeResult<T>;
}

/// A result type returned by Encoder operations.
pub type EncodeResult = Result<ByteVector, Error>;

/// A result type, consisting of a decoded value and any unconsumed data, returned by Decoder operations.
#[derive(Debug)]
pub struct DecoderResult<T> {
    pub value: T,
    pub remainder: ByteVector
}

/// A result type returned by Decoder operations.
pub type DecodeResult<T> = Result<DecoderResult<T>, Error>;

/// A reference to a Codec.
pub enum CodecRef<T: 'static> {
    /// An owned reference to a Codec that lives on the heap.
    Owned(Box<Codec<T>>),
    /// A reference to a static Codec instance.
    Static(&'static Codec<T>)
}
impl<T: 'static> Codec<T> for CodecRef<T> {
    fn encode(&self, value: &T) -> EncodeResult {
        match *self {
            CodecRef::Owned(ref c) => c.encode(value),
            CodecRef::Static(ref c) => c.encode(value)
        }
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<T> {
        match *self {
            CodecRef::Owned(ref c) => c.decode(bv),
            CodecRef::Static(ref c) => c.decode(bv)
        }
    }
}

/// Trait that enables limited function overloading, so that functions that accept one or more Codecs can
/// accept them as either a boxed value (Box<Codec<T>>) or a static reference (&'static Codec<T>).
pub trait AsCodecRef<T> {
    fn as_codec_ref(self) -> CodecRef<T>;
}
impl<T> AsCodecRef<T> for Box<Codec<T>> {
    fn as_codec_ref(self) -> CodecRef<T> {
        CodecRef::Owned(self)
    }
}
impl<T> AsCodecRef<T> for &'static Codec<T> {
    fn as_codec_ref(self) -> CodecRef<T> {
        CodecRef::Static(self)
    }
}

/// Unsigned 8-bit integer codec.
pub static uint8: &'static Codec<u8> = &IntegralCodec { order: ByteOrder::Big };

/// Signed 8-bit integer codec.
pub static int8: &'static Codec<i8> = &IntegralCodec { order: ByteOrder::Big };

/// Big-endian unsigned 16-bit integer codec.
pub static uint16: &'static Codec<u16> = &IntegralCodec { order: ByteOrder::Big };

/// Big-endian signed 16-bit integer codec.
pub static int16: &'static Codec<i16> = &IntegralCodec { order: ByteOrder::Big };

/// Big-endian unsigned 32-bit integer codec.
pub static uint32: &'static Codec<u32> = &IntegralCodec { order: ByteOrder::Big };

/// Big-endian signed 32-bit integer codec.
pub static int32: &'static Codec<i32> = &IntegralCodec { order: ByteOrder::Big };

/// Big-endian unsigned 64-bit integer codec.
pub static uint64: &'static Codec<u64> = &IntegralCodec { order: ByteOrder::Big };

/// Big-endian signed 64-bit integer codec.
pub static int64: &'static Codec<i64> = &IntegralCodec { order: ByteOrder::Big };

/// Little-endian unsigned 16-bit integer codec.
pub static uint16_l: &'static Codec<u16> = &IntegralCodec { order: ByteOrder::Little };

/// Little-endian signed 16-bit integer codec.
pub static int16_l: &'static Codec<i16> = &IntegralCodec { order: ByteOrder::Little };

/// Little-endian unsigned 32-bit integer codec.
pub static uint32_l: &'static Codec<u32> = &IntegralCodec { order: ByteOrder::Little };

/// Little-endian signed 32-bit integer codec.
pub static int32_l: &'static Codec<i32> = &IntegralCodec { order: ByteOrder::Little };

/// Little-endian unsigned 64-bit integer codec.
pub static uint64_l: &'static Codec<u64> = &IntegralCodec { order: ByteOrder::Little };

/// Little-endian signed 64-bit integer codec.
pub static int64_l: &'static Codec<i64> = &IntegralCodec { order: ByteOrder::Little };

/// A byte order specification, either big or little endian.
enum ByteOrder {
    Big,
    Little
}

struct IntegralCodec { order: ByteOrder }
impl<T: Int + FromPrimitive> Codec<T> for IntegralCodec {
    fn encode(&self, value: &T) -> EncodeResult {
        // TODO: Use direct() once it's implemented
        let size = size_of::<T>();
        let mut v = Vec::<u8>::with_capacity(size);
        for i in 0..size {
            let shift = match self.order {
                ByteOrder::Big => (size - i - 1) * 8,
                ByteOrder::Little => i * 8
            };
            let byte: u8 = match size {
                1 => {
                    // i8 requires special handling since it can't represent 0xff.
                    // We do the same calculations, but using i16 to avoid overflow.
                    // This same code path also works for u8, so we can just check the size.
                    let bigger: i16 = num::cast(*value).unwrap();
                    num::cast(bigger & 0xff).unwrap()
                }
                _ => num::cast((*value >> shift) & T::from_u8(0xff).unwrap()).unwrap()
            };
            v.push(byte);
        }
        Ok(byte_vector::buffered(&v))
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<T> {
        let size = size_of::<T>();
        let v = &mut vec::from_elem(0u8, size);
        bv.read(v, 0, size).and_then(|_size| {
            let value = match size {
                1 => {
                    // i8 requires special handling since it can't hold values >= 128.
                    // We convert the byte value to an i16 and then work with it to
                    // get around that limitation. Interpretation as two's complement
                    // is done manually by subtracting if the type is signed and
                    // the unsigned byte value is in the upper (negative) half of
                    // the range.
                    let mut value: i16 = num::cast(v[0]).unwrap();
                    let tmax: i16 = num::cast(T::max_value()).unwrap();
                    if value > tmax {
                        // This condition is only true if T is i8 and the value
                        // is negative. Convert from the serialized unsigned byte
                        // to the deserialized signed integer by subtracting 2^8.
                        value -= 256;
                    }
                    num::cast(value).unwrap()
                }
                _ => {
                    let mut value = T::zero();
                    for i in 0..size {
                        let byte = T::from_u8(v[i]).unwrap();
                        value = match self.order {
                            ByteOrder::Big => (value << 8) | byte,
                            ByteOrder::Little => value | (byte << (i * 8))
                        };
                    }
                    value
                }
            };
            bv.drop(size).map(|remainder| {
                DecoderResult { value: value, remainder: remainder }
            })
        })
    }
}

/// Codec that encodes `len` low bytes and decodes by discarding `len` bytes.
pub fn ignore(len: usize) -> Box<Codec<()>> {
    Box::new(IgnoreCodec { len: len })
}
struct IgnoreCodec { len: usize }
impl Codec<()> for IgnoreCodec {
    fn encode(&self, _value: &()) -> EncodeResult {
        Ok(byte_vector::fill(0, self.len))
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<()> {
        bv.drop(self.len).map(|remainder| {
            DecoderResult { value: (), remainder: remainder }
        })
    }
}

/// Codec that always encodes the given byte vector, and decodes by returning a unit result if the actual bytes match
/// the given byte vector or an error otherwise.
pub fn constant(bytes: &ByteVector) -> Box<Codec<()>> {
    Box::new(ConstantCodec { bytes: (*bytes).clone() })
}
struct ConstantCodec { bytes: ByteVector }
impl Codec<()> for ConstantCodec {
    fn encode(&self, _value: &()) -> EncodeResult {
        Ok(self.bytes.clone())
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<()> {
        bv.take(self.bytes.length()).and_then(|taken| {
            if taken == self.bytes {
                Ok(DecoderResult { value: (), remainder: bv.drop(self.bytes.length()).unwrap() })
            } else {
                Err(Error::new(format!("Expected constant {:?} but got {:?}", self.bytes, taken)))
            }
        })
    }
}

/// Identity byte vector codec.
///   - Encodes by returning the given byte vector.
///   - Decodes by taking all remaining bytes from the given byte vector.
pub static identity_bytes: &'static Codec<ByteVector> = &IdentityCodec;
struct IdentityCodec;
impl Codec<ByteVector> for IdentityCodec {
    fn encode(&self, value: &ByteVector) -> EncodeResult {
        Ok((*value).clone())
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<ByteVector> {
        Ok(DecoderResult { value: (*bv).clone(), remainder: byte_vector::empty() })
    }
}

/// Byte vector codec.
///   - Encodes by returning the given byte vector if its length is `len` bytes, otherwise returns an error.
///   - Decodes by taking `len` bytes from the given byte vector.
pub fn bytes(len: usize) -> Box<Codec<ByteVector>> {
    fixed_size_bytes(len, identity_bytes)
}

/// Codec that limits the number of bytes that are available to `codec`.
///
/// When encoding, if the given `codec` encodes fewer than `len` bytes, the byte vector
/// is right padded with low bytes.  If `codec` instead encodes more than `len` bytes,
/// an error is returned.
///
/// When decoding, the given `codec` is only given `len` bytes.  If `codec` does
/// not consume all `len` bytes, any remaining bytes are discarded.
pub fn fixed_size_bytes<T: 'static, TC: AsCodecRef<T>>(len: usize, codec: TC) -> Box<Codec<T>> {
    Box::new(FixedSizeCodec { len: len, codec: codec.as_codec_ref() })
}
struct FixedSizeCodec<T: 'static> { len: usize, codec: CodecRef<T> }
impl<T> Codec<T> for FixedSizeCodec<T> {
    fn encode(&self, value: &T) -> EncodeResult {
        self.codec.encode(value).and_then(|encoded| {
            if encoded.length() > self.len {
                Err(Error::new(format!("Encoding requires {} bytes but codec is limited to fixed length of {}", encoded.length(), self.len)))
            } else {
                encoded.pad_right(self.len)
            }
        })
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<T> {
        // Give `len` bytes to the decoder; if successful, return the result along with
        // the remainder of `bv` after dropping `len` bytes from it
        forcomp!({
            taken <- bv.take(self.len);
            decoded <- self.codec.decode(&taken);
        } yield {
            DecoderResult { value: decoded.value, remainder: bv.drop(self.len).unwrap() }
        })
    }
}

/// Codec for length-delimited values.
///   - Encodes by encoding the length (in bytes) of the value followed by the value itself.
///   - Decodes by decoding the length and then attempting to decode the value that follows.
pub fn variable_size_bytes<L: 'static + Int + ToPrimitive + FromPrimitive + Display, V: 'static, LC: AsCodecRef<L>, VC: AsCodecRef<V>>(len_codec: LC, val_codec: VC) -> Box<Codec<V>> {
    Box::new(VariableSizeCodec { len_codec: len_codec.as_codec_ref(), val_codec: val_codec.as_codec_ref() })
}
struct VariableSizeCodec<L: 'static + Int + ToPrimitive + FromPrimitive + Display, V: 'static> { len_codec: CodecRef<L>, val_codec: CodecRef<V> }
impl<L: Int + ToPrimitive + FromPrimitive + Display, V> Codec<V> for VariableSizeCodec<L, V> {
    // TODO: Currently there is no Unsigned trait that we can use to restrict the length codec to unsigned types,
    // but there is one proposed here:
    //   https://github.com/rust-lang/rfcs/blob/master/text/0369-num-reform.md
    // So if that ever comes to fruition, we should switch to that trait here in place of Int.
    
    fn encode(&self, value: &V) -> EncodeResult {
        // Encode the value, then prepend the length of the encoded value
        self.val_codec.encode(&value).and_then(|encoded_val| {
            // Fail if length is too long to be encoded
            match L::from_usize(encoded_val.length()) {
                Some(len) => self.len_codec.encode(&len).map(|encoded_len| byte_vector::append(&encoded_len, &encoded_val)),
                None => Err(Error::new(format!("Length of encoded value ({} bytes) is greater than maximum value ({}) of length type", encoded_val.length(), L::max_value())))
            }
        })
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<V> {
        // Decode the length, then decode the value
        forcomp!({
            decoded_len <- self.len_codec.decode(&bv);
            remainder <- {
                // TODO: Ideally we'd just use fixed_size_bytes() here, but not sure how to transfer ownership of val_decoder
                let len = decoded_len.value.to_usize().unwrap();
                decoded_len.remainder.take(len)
            };
            decoded_val <- self.val_codec.decode(&remainder);
        } yield {
            DecoderResult { value: decoded_val.value, remainder: bv.drop(remainder.length()).unwrap() }
        })
    }
}

/// Codec for HNil type.
pub static hnil_codec: &'static Codec<HNil> = &HNilCodec;
struct HNilCodec;
impl Codec<HNil> for HNilCodec {
    fn encode(&self, _value: &HNil) -> EncodeResult {
        Ok(byte_vector::empty())
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<HNil> {
        Ok(DecoderResult { value: HNil, remainder: bv.clone() })
    }
}

/// Codec used to convert an HList of codecs into a single codec that encodes/decodes an HList of values.
pub fn hlist_prepend_codec<H: 'static, T: 'static + HList, HC: AsCodecRef<H>, TC: AsCodecRef<T>>(head_codec: HC, tail_codec: TC) -> Box<Codec<HCons<H, T>>> {
    Box::new(HListPrependCodec { head_codec: head_codec.as_codec_ref(), tail_codec: tail_codec.as_codec_ref() })
}
struct HListPrependCodec<H: 'static, T: 'static + HList> { head_codec: CodecRef<H>, tail_codec: CodecRef<T> }
impl<H, T: HList> Codec<HCons<H, T>> for HListPrependCodec<H, T> {
    fn encode(&self, value: &HCons<H, T>) -> EncodeResult {
        // TODO: Generalize this as an encode_both() function
        forcomp!({
            encoded_head <- self.head_codec.encode(&value.head());
            encoded_tail <- self.tail_codec.encode(&value.tail());
        } yield {
            byte_vector::append(&encoded_head, &encoded_tail)
        })
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<HCons<H, T>> {
        // TODO: Generalize this as a decode_both_combine() function
        forcomp!({
            decoded_head <- self.head_codec.decode(&bv);
            decoded_tail <- self.tail_codec.decode(&decoded_head.remainder);
        } yield {
            DecoderResult { value: HCons(decoded_head.value, decoded_tail.value), remainder: decoded_tail.remainder }
        })
    }
}

/// Trait implemented by structs created by the record_struct! macro.
pub trait AsHList<T> {
    fn from_hlist(hlist: &T) -> Self;
    fn to_hlist(&self) -> T;
}

/// Codec for structs created by the record_struct! macro.
pub fn struct_codec<H: 'static, S: AsHList<H>, HC: AsCodecRef<H>>(hlist_codec: HC) -> Box<Codec<S>> {
    Box::new(RecordStructCodec { hlist_codec: hlist_codec.as_codec_ref() })
}
struct RecordStructCodec<H: 'static> { hlist_codec: CodecRef<H> }
impl<H, S: AsHList<H>> Codec<S> for RecordStructCodec<H> {
    fn encode(&self, value: &S) -> EncodeResult {
        self.hlist_codec.encode(&value.to_hlist())
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<S> {
        self.hlist_codec.decode(bv).map(|decoded| {
            DecoderResult { value: S::from_hlist(&decoded.value), remainder: decoded.remainder }
        })
    }
}

/// Override for the '|' operator that creates a new codec that injects additional context (e.g. in error messages)
/// into the codec on the right-hand side.
// TODO: Can we have a single impl that works on AsCodecRef<T>?  Attempts so far like this:
//   impl<T: 'static, TC: AsCodecRef<T>> core::ops::BitOr<TC> for &'static str {
// have resulted in:
//   error: the type parameter `T` is not constrained by the impl trait, self type, or predicates [E0207]
impl<T: 'static> core::ops::BitOr<Box<Codec<T>>> for &'static str {
    type Output = Box<Codec<T>>;

    fn bitor(self, rhs: Box<Codec<T>>) -> Box<Codec<T>> {
        Box::new(ContextCodec { codec: rhs.as_codec_ref(), context: self })
    }
}
impl<T: 'static> core::ops::BitOr<&'static Codec<T>> for &'static str {
    type Output = Box<Codec<T>>;

    fn bitor(self, rhs: &'static Codec<T>) -> Box<Codec<T>> {
        Box::new(ContextCodec { codec: rhs.as_codec_ref(), context: self })
    }
}
struct ContextCodec<T: 'static> { codec: CodecRef<T>, context: &'static str }
impl<T> Codec<T> for ContextCodec<T> {
    fn encode(&self, value: &T) -> EncodeResult {
        self.codec.encode(value).map_err(|e| e.push_context(self.context))
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<T> {
        self.codec.decode(bv).map_err(|e| e.push_context(self.context))
    }
}

/// Returns a new codec that encodes/decodes the unit value followed by the right-hand value,
/// discarding the unit value when decoding.
pub fn drop_left<T: 'static, LC: AsCodecRef<()>, RC: AsCodecRef<T>>(lhs: LC, rhs: RC) -> Box<Codec<T>> {
    Box::new(DropLeftCodec { lhs: lhs.as_codec_ref(), rhs: rhs.as_codec_ref() })
}
struct DropLeftCodec<T: 'static> { lhs: CodecRef<()>, rhs: CodecRef<T> }
impl<T> Codec<T> for DropLeftCodec<T> {
    fn encode(&self, value: &T) -> EncodeResult {
        forcomp!({
            encoded_lhs <- self.lhs.encode(&());
            encoded_rhs <- self.rhs.encode(value);
        } yield {
            byte_vector::append(&encoded_lhs, &encoded_rhs)
        })
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<T> {
        self.lhs.decode(bv).and_then(|decoded| {
            self.rhs.decode(&decoded.remainder)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;
    use error::Error;
    use byte_vector;
    use byte_vector::ByteVector;
    use hlist::*;

    #[test]
    fn forcomp_macro_should_work() {
        let v1 = forcomp!({
            foo <- Some(1u8);
        } yield { foo });
        assert!(v1.is_some());

        let v2 = forcomp!({
            foo <- Some(1u8);
            bar <- None;
        } yield { foo + bar });
        assert!(v2.is_none());

        let v3 = forcomp!({
            foo <- Some(1u8);
            bar <- Some(2u8);
        } yield { foo + bar });
        assert_eq!(v3.unwrap(), 3u8);
    }
    
    fn assert_round_trip_bytes<T: 'static + Eq + Debug, C: AsCodecRef<T>>(c: C, value: &T, raw_bytes: &Option<ByteVector>) {
        // Encode
        let codec = c.as_codec_ref();
        let result = codec.encode(value).and_then(|encoded| {
            // Compare encoded bytes to the expected bytes, if provided
            let compare_result = match *raw_bytes {
                Some(ref expected) => {
                    if encoded != *expected {
                        Err(Error::new(format!("Encoded bytes {:?} do not match expected bytes {:?}", encoded, *expected)))
                    } else {
                        Ok(())
                    }
                },
                None => Ok(())
            };
            if compare_result.is_err() {
                return Err(compare_result.unwrap_err());
            }
            
            // Decode and drop the remainder
            codec.decode(&encoded).map(|decoded| decoded.value)
        });

        // Verify result
        match result {
            Ok(decoded) => assert_eq!(decoded, *value),
            Err(e) => panic!("Round-trip encoding failed: {}", e.message()),
        }
    }

    //
    // Integral codecs
    // 
    
    #[test]
    fn a_u8_value_should_round_trip() {
        assert_round_trip_bytes(uint8, &7, &Some(byte_vector!(7)));
    }
    
    #[test]
    fn an_i8_value_should_round_trip() {
        assert_round_trip_bytes(int8, &7, &Some(byte_vector!(7)));
        assert_round_trip_bytes(int8, &-2, &Some(byte_vector!(0xfe)));
        assert_round_trip_bytes(int8, &-16, &Some(byte_vector!(0xf0)));
        assert_round_trip_bytes(int8, &-128, &Some(byte_vector!(0x80)));
    }
    
    #[test]
    fn a_u16_value_should_round_trip() {
        assert_round_trip_bytes(uint16, &0x1234, &Some(byte_vector!(0x12, 0x34)));
        assert_round_trip_bytes(uint16_l, &0x1234, &Some(byte_vector!(0x34, 0x12)));
    }

    #[test]
    fn an_i16_value_should_round_trip() {
        assert_round_trip_bytes(int16, &0x1234, &Some(byte_vector!(0x12, 0x34)));
        assert_round_trip_bytes(int16, &-2, &Some(byte_vector!(0xff, 0xfe)));
        assert_round_trip_bytes(int16_l, &0x1234, &Some(byte_vector!(0x34, 0x12)));
        assert_round_trip_bytes(int16_l, &-2, &Some(byte_vector!(0xfe, 0xff)));
    }

    #[test]
    fn a_u32_value_should_round_trip() {
        assert_round_trip_bytes(uint32, &0x12345678, &Some(byte_vector!(0x12, 0x34, 0x56, 0x78)));
        assert_round_trip_bytes(uint32_l, &0x12345678, &Some(byte_vector!(0x78, 0x56, 0x34, 0x12)));
    }

    #[test]
    fn an_i32_value_should_round_trip() {
        assert_round_trip_bytes(uint32, &0x12345678, &Some(byte_vector!(0x12, 0x34, 0x56, 0x78)));
        assert_round_trip_bytes(uint32, &-2, &Some(byte_vector!(0xff, 0xff, 0xff, 0xfe)));
        assert_round_trip_bytes(uint32_l, &0x12345678, &Some(byte_vector!(0x78, 0x56, 0x34, 0x12)));
        assert_round_trip_bytes(uint32_l, &-2, &Some(byte_vector!(0xfe, 0xff, 0xff, 0xff)));
    }

    #[test]
    fn a_u64_value_should_round_trip() {
        assert_round_trip_bytes(uint64, &0x1234567890abcdef, &Some(byte_vector!(0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef)));
        assert_round_trip_bytes(uint64_l, &0x1234567890abcdef, &Some(byte_vector!(0xef, 0xcd, 0xab, 0x90, 0x78, 0x56, 0x34, 0x12)));
    }

    #[test]
    fn an_i64_value_should_round_trip() {
        assert_round_trip_bytes(int64, &0x1234567890abcdef, &Some(byte_vector!(0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef)));
        assert_round_trip_bytes(int64, &-2, &Some(byte_vector!(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe)));
        assert_round_trip_bytes(int64_l, &0x1234567890abcdef, &Some(byte_vector!(0xef, 0xcd, 0xab, 0x90, 0x78, 0x56, 0x34, 0x12)));
        assert_round_trip_bytes(int64_l, &-2, &Some(byte_vector!(0xfe, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)));
    }

    //
    // Ignore codec
    // 
    
    #[test]
    fn an_ignore_codec_should_round_trip() {
        assert_round_trip_bytes(ignore(4), &(), &Some(byte_vector!(0, 0, 0, 0)));
    }

    #[test]
    fn decoding_with_ignore_codec_should_succeed_if_the_input_vector_is_long_enough() {
        let input = byte_vector!(7, 1, 2, 3, 4);
        let codec = ignore(3);
        match codec.decode(&input) {
            Ok(result) => {
                let expected_remainder = byte_vector!(3, 4);
                assert_eq!(expected_remainder, result.remainder);
            },
            Err(e) => panic!("Decoding failed: {}", e.message())
        }
    }

    #[test]
    fn decoding_with_ignore_codec_should_fail_if_the_input_vector_is_smaller_than_the_ignored_length() {
        let input = byte_vector!(1u8);
        let codec = ignore(3);
        assert_eq!(codec.decode(&input).unwrap_err().message(), "Requested length of 3 bytes exceeds vector length of 1");
    }

    //
    // Constant codec
    // 

    #[test]
    fn a_constant_codec_should_round_trip() {
        let input = byte_vector!(1, 2, 3, 4);
        assert_round_trip_bytes(constant(&input), &(), &Some(input));
    }

    #[test]
    fn decoding_with_constant_codec_should_fail_if_the_input_vector_does_not_match_the_constant_vector() {
        let input = byte_vector!(1, 2, 3, 4);
        let codec = constant(&byte_vector!(6, 6, 6));
        assert_eq!(codec.decode(&input).unwrap_err().message(), "Expected constant 060606 but got 010203");
    }

    #[test]
    fn decoding_with_constant_codec_should_fail_if_the_input_vector_is_smaller_than_the_constant_vector() {
        let input = byte_vector!(1);
        let codec = constant(&byte_vector!(6, 6, 6));
        assert_eq!(codec.decode(&input).unwrap_err().message(), "Requested view offset of 0 and length 3 bytes exceeds vector length of 1");
    }

    //
    // Identity codec
    //
    
    #[test]
    fn an_identity_codec_should_round_trip() {
        let input = byte_vector!(1, 2, 3, 4);
        assert_round_trip_bytes(identity_bytes, &input, &Some(input.clone()));
    }

    //
    // Bytes codec
    //

    #[test]
    fn a_byte_vector_codec_should_round_trip() {
        let input = byte_vector!(7, 1, 2, 3, 4);
        assert_round_trip_bytes(bytes(5), &input, &Some(input.clone()));
    }

    #[test]
    fn decoding_with_byte_vector_codec_should_return_remainder_that_had_len_bytes_dropped() {
        let input = byte_vector!(7, 1, 2, 3, 4);
        let codec = bytes(3);
        match codec.decode(&input) {
            Ok(result) => {
                assert_eq!(result.value, byte_vector!(7, 1, 2));
                assert_eq!(result.remainder, byte_vector!(3, 4));
            },
            Err(e) => panic!("Decoding failed: {}", e.message())
        }
    }

    #[test]
    fn decoding_with_byte_vector_codec_should_fail_when_vector_has_less_space_than_given_length() {
        let input = byte_vector!(1, 2);
        let codec = bytes(4);
        assert_eq!(codec.decode(&input).unwrap_err().message(), "Requested view offset of 0 and length 4 bytes exceeds vector length of 2");
    }

    //
    // Fixed size bytes codec
    //

    #[test]
    fn a_fixed_size_bytes_codec_should_round_trip() {
        let codec = fixed_size_bytes(1, uint8);
        assert_round_trip_bytes(codec, &7u8, &Some(byte_vector!(7)));
    }

    #[test]
    fn encoding_with_fixed_size_codec_should_pad_with_zeros_when_value_is_smaller_than_given_length() {
        let codec = fixed_size_bytes(3, uint8);
        assert_round_trip_bytes(codec, &7u8, &Some(byte_vector!(7, 0, 0)));
    }

    #[test]
    fn encoding_with_fixed_size_codec_should_fail_when_value_needs_more_space_than_given_length() {
        let codec = fixed_size_bytes(1, constant(&byte_vector!(6, 6, 6)));
        assert_eq!(codec.encode(&()).unwrap_err().message(), "Encoding requires 3 bytes but codec is limited to fixed length of 1");
    }

    #[test]
    fn decoding_with_fixed_size_codec_should_return_remainder_that_had_len_bytes_dropped() {
        let input = byte_vector!(7, 1, 2, 3, 4);
        let codec = fixed_size_bytes(3, uint8);
        match codec.decode(&input) {
            Ok(result) => {
                assert_eq!(result.value, 7u8);
                assert_eq!(result.remainder, byte_vector!(3, 4));
            },
            Err(e) => panic!("Decoding failed: {}", e.message())
        }
    }
    
    #[test]
    fn decoding_with_fixed_size_codec_should_fail_when_vector_has_less_space_than_given_length() {
        let input = byte_vector!(1, 2);
        let codec = fixed_size_bytes(4, bytes(6));
        assert_eq!(codec.decode(&input).unwrap_err().message(), "Requested view offset of 0 and length 4 bytes exceeds vector length of 2");
    }

    //
    // Variable size bytes codec
    //

    #[test]
    fn a_variable_size_bytes_codec_should_round_trip() {
        let input = byte_vector!(7, 1, 2, 3, 4);
        let codec = variable_size_bytes(uint16, identity_bytes);
        assert_round_trip_bytes(codec, &input, &Some(byte_vector!(0, 5, 7, 1, 2, 3, 4)));
    }

    #[test]
    fn encoding_with_variable_size_codec_should_fail_when_length_of_encoded_value_is_too_large() {
        let input = byte_vector::fill(0x7, 256);
        let codec = variable_size_bytes(uint8, identity_bytes);
        assert_eq!(codec.encode(&input).unwrap_err().message(), "Length of encoded value (256 bytes) is greater than maximum value (255) of length type");
    }

    //
    // Context injection ('|' operator)
    //
    
    #[allow(unused_parens)]
    #[test]
    fn context_should_be_pushed_when_using_the_bitor_operator() {
        let input = byte_vector::empty();
        let codec =
            ("section" |
             ("header" |
              ("magic" | uint8)
              )
             );

        // Verify that the error message is prefexed with the correct context
        assert_eq!(codec.decode(&input).unwrap_err().message(), "section/header/magic: Requested read offset of 0 and length 1 bytes exceeds vector length of 0");
    }

    //
    // HList-related codecs
    //
    
    #[test]
    fn an_hnil_codec_should_round_trip() {
        assert_round_trip_bytes(hnil_codec, &HNil, &Some(byte_vector::empty()));
    }

    #[test]
    fn an_hlist_prepend_codec_should_round_trip() {
        let codec1 = hlist_prepend_codec(uint8, hnil_codec);
        assert_round_trip_bytes(codec1, &hlist!(7u8), &Some(byte_vector!(7)));

        let codec2 = hlist_prepend_codec(uint8, hlist_prepend_codec(uint8, hnil_codec));
        assert_round_trip_bytes(codec2, &hlist!(7u8, 3u8), &Some(byte_vector!(7, 3)));
    }

    #[test]
    fn an_hlist_codec_should_round_trip() {
        let codec = hcodec!({uint8} :: {uint8} :: {uint8}); 
        assert_round_trip_bytes(codec, &hlist!(7u8, 3u8, 1u8), &Some(byte_vector!(7, 3, 1)));
    }

    #[test]
    fn the_hcodec_macro_should_work_with_context_injected_codecs() {
        let m = byte_vector!(0xCA, 0xFE);
        let codec = hcodec!(
            { "magic"  | constant(&m) } >>
            { "first"  | uint8        } ::
            { "trash"  | ignore(1)    } >>
            { "second" | uint8        } :: 
            { "third"  | uint8        }
        );
        
        let input = hlist!(7u8, 3u8, 1u8);
        let expected = byte_vector!(0xCA, 0xFE, 0x07, 0x00, 0x03, 0x01);
        assert_round_trip_bytes(codec, &input, &Some(expected));
    }

    //
    // Struct conversion codec
    //
    
    record_struct_with_hlist_type!(
        TestStruct1, HCons<u8, HCons<u8, HNil>>,
        foo: u8,
        bar: u8);

    record_struct!(
        TestStruct2,
        foo: u8,
        bar: u8);

    #[test]
    fn record_structs_should_work() {
        let hlist = hlist!(7u8, 3u8);
        let s1 = TestStruct1::from_hlist(&hlist);
        let s2 = TestStruct2::from_hlist(&hlist);
        assert_eq!(s1.foo, s2.foo);
        assert_eq!(s1.bar, s2.bar);
    }

    #[test]
    fn a_struct_codec_should_round_trip() {
        let codec = struct_codec!(TestStruct2 from {uint8} :: {uint8});
        assert_round_trip_bytes(codec, &TestStruct2 { foo: 7u8, bar: 3u8 }, &Some(byte_vector!(7, 3)));
    }
}
