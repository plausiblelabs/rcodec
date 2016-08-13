//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

// The following allows for non-uppercase constants (e.g. uint32_l vs UINT32_L).
#![allow(non_upper_case_globals)]

use std::fmt::Display;
use std::marker::PhantomData;
use std::mem::size_of;
use std::ptr;
use std::slice;

use num::traits::{PrimInt, Unsigned, FromPrimitive};

use error::Error;
use byte_vector;
use byte_vector::ByteVector;
use hlist::*;

/// Implements encoding and decoding of values of type `Value`.
pub trait Codec {
    /// The value type.
    type Value;
    
    /// Attempts to encode a value of type `Value` into a `ByteVector`.
    fn encode(&self, value: &Self::Value) -> EncodeResult;
    
    /// Attempts to decode a value of type `Value` from the given `ByteVector`.
    fn decode(&self, bv: &ByteVector) -> DecodeResult<Self::Value>;
}

/// A result type returned by `encode` operations.
pub type EncodeResult = Result<ByteVector, Error>;

/// A result type, consisting of a decoded value and any unconsumed data, returned by `decode` operations.
#[derive(Debug)]
pub struct DecoderResult<V> {
    /// The decoded value.
    pub value: V,

    /// The unconsumed data.
    pub remainder: ByteVector
}

/// A result type returned by `decode` operations.
pub type DecodeResult<V> = Result<DecoderResult<V>, Error>;

// Automatically provides implementation of `Codec` trait for all `Box<Codec>`.
impl<C: Codec + ?Sized> Codec for Box<C> {
    type Value = C::Value;

    #[inline(always)]
    fn encode(&self, value: &Self::Value) -> EncodeResult {
        (**self).encode(value)
    }
    
    #[inline(always)]
    fn decode(&self, bv: &ByteVector) -> DecodeResult<Self::Value> {
        (**self).decode(bv)
    }
}

// Automatically provides implementation of `Codec` trait for all `&'static Codec`.
impl<C: Codec + ?Sized> Codec for &'static C {
    type Value = C::Value;

    #[inline(always)]
    fn encode(&self, value: &Self::Value) -> EncodeResult {
        (*self).encode(value)
    }
    
    #[inline(always)]
    fn decode(&self, bv: &ByteVector) -> DecodeResult<Self::Value> {
        (*self).decode(bv)
    }
}



//
// Integral codecs
// 

macro_rules! integral_codec {
    { $structname:ident, $value:ident, $encswap:expr, $decswap:expr } => {
        /// Codec for primitive integral types.
        #[doc(hidden)]
        pub struct $structname<T> {
            _marker: PhantomData<T>
        }

        impl<T> Codec for $structname<T>
            where T: PrimInt
        {
            type Value = T;
            
            fn encode(&self, $value: &T) -> EncodeResult {
                let size = size_of::<T>();
                let mut v = [0u8; byte_vector::DIRECT_VALUE_SIZE_LIMIT];
                unsafe {
                    let src_ptr: *const u8 = ($encswap as *const T) as *const u8;
                    let dst_ptr: *mut u8 = v.as_mut_ptr();
                    ptr::copy(src_ptr, dst_ptr, size);
                }
                Ok(byte_vector::from_slice(v, size))
            }

            fn decode(&self, bv: &ByteVector) -> DecodeResult<T> {
                let size = size_of::<T>();
                let mut $value: T = T::zero();
                return unsafe {
                    let dst_ptr: *mut u8 = (&mut $value as *mut T) as *mut u8;
                    let mut buf = slice::from_raw_parts_mut(dst_ptr, size);
                    bv.read(&mut buf, 0, size).and_then(|_size| {
                        bv.drop(size).map(|remainder| {
                            DecoderResult { value: $decswap, remainder: remainder }
                        })
                    })
                }
            }
        }
    }
}

integral_codec!(IntegralCodec, value, value, value);
integral_codec!(IntegralBECodec, value, &(*value).to_be(), value.to_be());
integral_codec!(IntegralLECodec, value, &(*value).to_le(), value.to_le());

/// Unsigned 8-bit integer codec.    
pub const uint8: &'static Codec<Value=u8> = &IntegralCodec { _marker: PhantomData::<u8> };

/// Signed 8-bit integer codec.
pub const int8: &'static Codec<Value=i8> = &IntegralCodec { _marker: PhantomData::<i8> };

/// Big-endian unsigned 16-bit integer codec.
pub const uint16: &'static Codec<Value=u16> = &IntegralBECodec { _marker: PhantomData::<u16> };

/// Big-endian signed 16-bit integer codec.
pub const int16: &'static Codec<Value=i16> = &IntegralBECodec { _marker: PhantomData::<i16> };

/// Big-endian unsigned 32-bit integer codec.
pub const uint32: &'static Codec<Value=u32> = &IntegralBECodec { _marker: PhantomData::<u32> };

/// Big-endian signed 32-bit integer codec.
pub const int32: &'static Codec<Value=i32> = &IntegralBECodec { _marker: PhantomData::<i32> };

/// Big-endian unsigned 64-bit integer codec.
pub const uint64: &'static Codec<Value=u64> = &IntegralBECodec { _marker: PhantomData::<u64> };

/// Big-endian signed 64-bit integer codec.
pub const int64: &'static Codec<Value=i64> = &IntegralBECodec { _marker: PhantomData::<i64> };

/// Little-endian unsigned 16-bit integer codec.
pub const uint16_l: &'static Codec<Value=u16> = &IntegralLECodec { _marker: PhantomData::<u16> };

/// Little-endian signed 16-bit integer codec.
pub const int16_l: &'static Codec<Value=i16> = &IntegralLECodec { _marker: PhantomData::<i16> };

/// Little-endian unsigned 32-bit integer codec.
pub const uint32_l: &'static Codec<Value=u32> = &IntegralLECodec { _marker: PhantomData::<u32> };

/// Little-endian signed 32-bit integer codec.
pub const int32_l: &'static Codec<Value=i32> = &IntegralLECodec { _marker: PhantomData::<i32> };

/// Little-endian unsigned 64-bit integer codec.
pub const uint64_l: &'static Codec<Value=u64> = &IntegralLECodec { _marker: PhantomData::<u64> };

/// Little-endian signed 64-bit integer codec.
pub const int64_l: &'static Codec<Value=i64> = &IntegralLECodec { _marker: PhantomData::<i64> };



//
// Ignore codec
// 

/// Codec that encodes `len` low bytes and decodes by discarding `len` bytes.
#[inline(always)]
pub fn ignore(len: usize) -> IgnoreCodec {
    IgnoreCodec { len: len }
}

#[doc(hidden)]
pub struct IgnoreCodec {
    len: usize
}

impl Codec for IgnoreCodec {
    type Value = ();
    
    fn encode(&self, _value: &()) -> EncodeResult {
        Ok(byte_vector::fill(0, self.len))
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<()> {
        bv.drop(self.len).map(|remainder| {
            DecoderResult { value: (), remainder: remainder }
        })
    }
}



//
// Constant codec
// 

/// Codec that always encodes the given byte vector, and decodes by returning a unit result if the actual bytes match
/// the given byte vector or an error otherwise.
#[inline(always)]
pub fn constant(bytes: &ByteVector) -> ConstantCodec {
    ConstantCodec { bytes: (*bytes).clone() }
}

#[doc(hidden)]
pub struct ConstantCodec {
    bytes: ByteVector
}

impl Codec for ConstantCodec {
    type Value = ();
    
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



//
// Identity codec
// 

/// Identity byte vector codec.
///
///   - Encodes by returning the given byte vector.
///   - Decodes by taking all remaining bytes from the given byte vector.
#[inline(always)]
pub fn identity_bytes() -> IdentityCodec {
    IdentityCodec
}

#[doc(hidden)]
pub struct IdentityCodec;

impl Codec for IdentityCodec {
    type Value = ByteVector;
    
    fn encode(&self, value: &ByteVector) -> EncodeResult {
        Ok((*value).clone())
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<ByteVector> {
        Ok(DecoderResult { value: (*bv).clone(), remainder: byte_vector::empty() })
    }
}



//
// Bytes codec
// 

/// Byte vector codec.
///
///   - Encodes by returning the given byte vector if its length is `len` bytes, otherwise returns an error.
///   - Decodes by taking `len` bytes from the given byte vector.
#[inline(always)]
pub fn bytes(len: usize) -> FixedSizeCodec<IdentityCodec> {
    fixed_size_bytes(len, identity_bytes())
}



//
// Fixed size bytes codec
//

/// Codec that limits the number of bytes that are available to the given `codec`.
///
/// When encoding, if the given `codec` encodes fewer than `len` bytes, the byte vector
/// is right padded with low bytes.  If `codec` instead encodes more than `len` bytes,
/// an error is returned.
///
/// When decoding, the given `codec` is only given `len` bytes.  If `codec` does
/// not consume all `len` bytes, any remaining bytes are discarded.
#[inline(always)]
pub fn fixed_size_bytes<T, C>(len: usize, codec: C) -> FixedSizeCodec<C>
    where C: Codec<Value=T>
{
    FixedSizeCodec {
        len: len,
        codec: codec
    }
}

#[doc(hidden)]
pub struct FixedSizeCodec<C> {
    len: usize,
    codec: C
}

impl<T, C> Codec for FixedSizeCodec<C>
    where C: Codec<Value=T>
{
    type Value = T;
    
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



//
// Variable size bytes codec
//

/// Codec for length-delimited values.
///
///   - Encodes by encoding the length (in bytes) of the value followed by the value itself.
///   - Decodes by decoding the length and then attempting to decode the value that follows.
#[inline(always)]
pub fn variable_size_bytes<L, V, LC, VC>(len_codec: LC, val_codec: VC) -> VariableSizeCodec<LC, VC>
    where L: PrimInt + Unsigned + FromPrimitive + Display, LC: Codec<Value=L>, VC: Codec<Value=V>
{
    VariableSizeCodec {
        len_codec: len_codec,
        val_codec: val_codec
    }
}

#[doc(hidden)]
pub struct VariableSizeCodec<LC, VC> {
    len_codec: LC,
    val_codec: VC
}

impl<L, V, LC, VC> Codec for VariableSizeCodec<LC, VC>
    where L: PrimInt + Unsigned + FromPrimitive + Display, LC: Codec<Value=L>, VC: Codec<Value=V>
{
    type Value = V;
    
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



//
// Eager bytes codec
//

/// Codec that encodes/decodes fully-realized `Vec<u8>` values.
///
///   - Encodes by first efficiently converting `Vec<u8>` values to a `ByteVector`.
///   - Decodes by performing a fully-realized read on the backing `ByteVector`.
#[inline(always)]
pub fn eager<C>(bv_codec: C) -> EagerCodec<C>
    where C: Codec<Value=ByteVector>
{
    EagerCodec {
        bv_codec: bv_codec
    }
}
#[doc(hidden)]
pub struct EagerCodec<C> { bv_codec: C }
impl<C> Codec for EagerCodec<C>
    where C: Codec<Value=ByteVector>
{
    type Value = Vec<u8>;
    
    fn encode(&self, value: &Vec<u8>) -> EncodeResult {
        self.bv_codec.encode(&byte_vector::from_vec_copy(value))
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<Vec<u8>> {
        forcomp!({
            decoded <- self.bv_codec.decode(bv);
            vec <- decoded.value.to_vec();
        } yield {
            DecoderResult { value: vec, remainder: decoded.remainder }
        })
    }
}



//
// HList-related codecs
//

/// Codec for `HNil` type.
#[inline(always)]
pub fn hnil_codec() -> HNilCodec {
    HNilCodec
}

#[doc(hidden)]
pub struct HNilCodec;

impl Codec for HNilCodec {
    type Value = HNil;
    
    fn encode(&self, _value: &HNil) -> EncodeResult {
        Ok(byte_vector::empty())
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<HNil> {
        Ok(DecoderResult { value: HNil, remainder: bv.clone() })
    }
}

/// Codec used to convert an `HList` of codecs into a single codec that encodes/decodes an `HList` of values.
#[inline(always)]
pub fn hlist_prepend_codec<H, T, HC, TC>(head_codec: HC, tail_codec: TC) -> HListPrependCodec<HC, TC>
    where T: HList, HC: Codec<Value=H>, TC: Codec<Value=T>
{
    HListPrependCodec {
        head_codec: head_codec,
        tail_codec: tail_codec
    }
}

#[doc(hidden)]
pub struct HListPrependCodec<HC, TC> {
    head_codec: HC,
    tail_codec: TC
}

impl<H, T, HC, TC> Codec for HListPrependCodec<HC, TC>
    where T: HList, HC: Codec<Value=H>, TC: Codec<Value=T>
{
    type Value = HCons<H, T>;
    
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

/// Codec that first performs encoding/decoding of `T`, using the resulting value to produce codecs
/// for the remaining types.
///
/// This allows later parts of an `HList` codec to be dependent on on earlier values.
#[inline(always)]
pub fn hlist_flat_prepend_codec<H, T, HC, F>(head_codec: HC, tail_codec_fn: F) -> HListFlatPrependCodec<HC, F>
    where T: HList, HC: Codec<Value=H>, F: Fn(&H) -> Box<Codec<Value=T>>
{
    HListFlatPrependCodec {
        head_codec: head_codec,
        tail_codec_fn: tail_codec_fn
    }
}

#[doc(hidden)]
pub struct HListFlatPrependCodec<HC, F> {
    head_codec: HC,
    tail_codec_fn: F
}

impl<H, T, HC, F> Codec for HListFlatPrependCodec<HC, F>
    where T: HList, HC: Codec<Value=H>, F: Fn(&H) -> Box<Codec<Value=T>>
{
    type Value = HCons<H, T>;
    
    fn encode(&self, value: &HCons<H, T>) -> EncodeResult {
        // TODO: Generalize this as an encode_both() function
        forcomp!({
            encoded_head <- self.head_codec.encode(&value.head());
            encoded_tail <- (self.tail_codec_fn)(&value.head()).encode(&value.tail());
        } yield {
            byte_vector::append(&encoded_head, &encoded_tail)
        })
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<HCons<H, T>> {
        forcomp!({
            decoded_head <- self.head_codec.decode(&bv);
            decoded_tail <- (self.tail_codec_fn)(&decoded_head.value).decode(&decoded_head.remainder);
        } yield {
            DecoderResult { value: HCons(decoded_head.value, decoded_tail.value), remainder: decoded_tail.remainder }
        })
    }
}



//
// Struct codec
//

/// Codec for structs that support `HList` conversions.
#[inline(always)]
pub fn struct_codec<H, S, HC>(hlist_codec: HC) -> RecordStructCodec<S, HC>
    where H: HList, S: FromHList<H> + ToHList<H>, HC: Codec<Value=H>
{
    RecordStructCodec {
        hlist_codec: hlist_codec,
        _marker: PhantomData::<S>
    }
}

#[doc(hidden)]
pub struct RecordStructCodec<S, HC> {
    hlist_codec: HC,
    _marker: PhantomData<S>
}

impl<H, S, HC> Codec for RecordStructCodec<S, HC>
    where H: HList, S: FromHList<H> + ToHList<H>, HC: Codec<Value=H>
{
    type Value = S;
    
    fn encode(&self, value: &S) -> EncodeResult {
        self.hlist_codec.encode(&value.to_hlist())
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<S> {
        self.hlist_codec.decode(bv).map(|decoded| {
            DecoderResult { value: S::from_hlist(decoded.value), remainder: decoded.remainder }
        })
    }
}



//
// Context-injection codec
//

//
// TODO: Can we have a single impl that works on AsCodecRef<T>?  Attempts so far like this:
//   impl<T: 'static, TC: AsCodecRef<T>> core::ops::BitOr<TC> for &'static str {
//
// TODO: The orphan checking rules were changed shortly before Rust 1.0.0 such that we can't implement
// the BitOr trait with a Codec on the RHS.  Compilation fails with:
//
// src/codec.rs:475:1: 481:2 error: type parameter `T` must be used as the type parameter for some local type
//                           (e.g. `MyStruct<T>`); only traits defined in the current crate can be implemented
//                           for a type parameter [E0210]
// src/codec.rs:475 impl<T: 'static> core::ops::BitOr<RcCodec<T>> for &'static str {
// src/codec.rs:476     type Output = RcCodec<T>;
// src/codec.rs:477 
// src/codec.rs:478     fn bitor(self, rhs: RcCodec<T>) -> RcCodec<T> {
// src/codec.rs:479         rcbox!(ContextCodec { codec: rhs.as_codec_ref(), context: self })
// src/codec.rs:480     }
//
// See related discussion here:
//   https://github.com/rust-lang/rust/issues/20749
//
// As a workaround, we handle context injection directly inside the hcodec! macro, sigh.
//
// impl<T: 'static> core::ops::BitOr<&'static Codec<T>> for &'static str {
//     type Output = RcCodec<T>;

//     fn bitor(self, rhs: &'static Codec<T>) -> RcCodec<T> {
//         rcbox!(ContextCodec { codec: rhs.as_codec_ref(), context: self })
//     }
// }
// impl<T: 'static> core::ops::BitOr<RcCodec<T>> for &'static str {
//     type Output = RcCodec<T>;

//     fn bitor(self, rhs: RcCodec<T>) -> RcCodec<T> {
//         rcbox!(ContextCodec { codec: rhs.as_codec_ref(), context: self })
//     }
// }
/// Codec that injects additional context (e.g. in error messages) into the given codec.
#[inline(always)]
pub fn with_context<T, C>(context: &'static str, codec: C) -> ContextCodec<C>
    where C: Codec<Value=T>
{
    ContextCodec {
        codec: codec,
        context: context
    }
}

#[doc(hidden)]
pub struct ContextCodec<C> {
    codec: C,
    context: &'static str
}

impl<T, C> Codec for ContextCodec<C>
    where C: Codec<Value=T>
{
    type Value = T;
    
    fn encode(&self, value: &T) -> EncodeResult {
        self.codec.encode(value).map_err(|e| e.push_context(self.context))
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<T> {
        self.codec.decode(bv).map_err(|e| e.push_context(self.context))
    }
}



//
// Drop-left codec
//

/// Codec that encodes/decodes the unit value followed by the right-hand value, discarding
/// the unit value when decoding.
#[inline(always)]
pub fn drop_left<T, LC, RC>(lhs: LC, rhs: RC) -> DropLeftCodec<LC, RC>
    where LC: Codec<Value=()>, RC: Codec<Value=T>
{
    DropLeftCodec {
        lhs: lhs,
        rhs: rhs
    }
}

#[doc(hidden)]
pub struct DropLeftCodec<LC, RC> {
    lhs: LC,
    rhs: RC
}

impl<T, LC, RC> Codec for DropLeftCodec<LC, RC>
    where LC: Codec<Value=()>, RC: Codec<Value=T>
{
    type Value = T;
    
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
    use test::Bencher;
    use std::fmt::Debug;
    use std::marker::PhantomData;
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
            bar <- None::<u8>;
        } yield { foo + bar });
        assert!(v2.is_none());

        let v3 = forcomp!({
            foo <- Some(1u8);
            bar <- Some(2u8);
        } yield { foo + bar });
        assert_eq!(v3.unwrap(), 3u8);
    }
    
    fn assert_round_trip<T, C>(codec: C, value: &T, raw_bytes: &Option<ByteVector>)
        where T: 'static + Eq + Debug, C: Codec<Value=T>
    {
        // Encode
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
        assert_round_trip(uint8, &7, &Some(byte_vector!(7)));
    }

    #[test]
    fn an_i8_value_should_round_trip() {
        assert_round_trip(int8, &7, &Some(byte_vector!(7)));
        assert_round_trip(int8, &-2, &Some(byte_vector!(0xfe)));
        assert_round_trip(int8, &-16, &Some(byte_vector!(0xf0)));
        assert_round_trip(int8, &-128, &Some(byte_vector!(0x80)));
    }
    
    #[test]
    fn a_u16_value_should_round_trip() {
        assert_round_trip(uint16, &0x1234, &Some(byte_vector!(0x12, 0x34)));
        assert_round_trip(uint16_l, &0x1234, &Some(byte_vector!(0x34, 0x12)));
    }

    #[test]
    fn an_i16_value_should_round_trip() {
        assert_round_trip(int16, &0x1234, &Some(byte_vector!(0x12, 0x34)));
        assert_round_trip(int16, &-2, &Some(byte_vector!(0xff, 0xfe)));
        assert_round_trip(int16_l, &0x1234, &Some(byte_vector!(0x34, 0x12)));
        assert_round_trip(int16_l, &-2, &Some(byte_vector!(0xfe, 0xff)));
    }

    #[test]
    fn a_u32_value_should_round_trip() {
        assert_round_trip(uint32, &0x12345678, &Some(byte_vector!(0x12, 0x34, 0x56, 0x78)));
        assert_round_trip(uint32_l, &0x12345678, &Some(byte_vector!(0x78, 0x56, 0x34, 0x12)));
    }

    #[test]
    fn an_i32_value_should_round_trip() {
        assert_round_trip(int32, &0x12345678, &Some(byte_vector!(0x12, 0x34, 0x56, 0x78)));
        assert_round_trip(int32, &-2, &Some(byte_vector!(0xff, 0xff, 0xff, 0xfe)));
        assert_round_trip(int32_l, &0x12345678, &Some(byte_vector!(0x78, 0x56, 0x34, 0x12)));
        assert_round_trip(int32_l, &-2, &Some(byte_vector!(0xfe, 0xff, 0xff, 0xff)));
    }

    #[test]
    fn a_u64_value_should_round_trip() {
        assert_round_trip(uint64, &0x1234567890abcdef, &Some(byte_vector!(0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef)));
        assert_round_trip(uint64_l, &0x1234567890abcdef, &Some(byte_vector!(0xef, 0xcd, 0xab, 0x90, 0x78, 0x56, 0x34, 0x12)));
    }

    #[test]
    fn an_i64_value_should_round_trip() {
        assert_round_trip(int64, &0x1234567890abcdef, &Some(byte_vector!(0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef)));
        assert_round_trip(int64, &-2, &Some(byte_vector!(0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe)));
        assert_round_trip(int64_l, &0x1234567890abcdef, &Some(byte_vector!(0xef, 0xcd, 0xab, 0x90, 0x78, 0x56, 0x34, 0x12)));
        assert_round_trip(int64_l, &-2, &Some(byte_vector!(0xfe, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff)));
    }

    macro_rules! bench_int_codec {
        { $codec:ident, $enc:ident, $dec:ident } => {
            #[bench]
            fn $enc(b: &mut Bencher) {
                b.iter(|| $codec.encode(&7));
            }

            #[bench]
            fn $dec(b: &mut Bencher) {
                let bv = byte_vector!(0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07);
                b.iter(|| $codec.decode(&bv));
            }
        };
    }

    bench_int_codec!(uint8,    bench_enc_uint8,    bench_dec_uint8);
    bench_int_codec!(int8,     bench_enc_int8,     bench_dec_int8);

    bench_int_codec!(uint16,   bench_enc_uint16,   bench_dec_uint16);
    bench_int_codec!(int16,    bench_enc_int16,    bench_dec_int16);
    bench_int_codec!(uint16_l, bench_enc_uint16_l, bench_dec_uint16_l);
    bench_int_codec!(int16_l,  bench_enc_int16_l,  bench_dec_int16_l);

    bench_int_codec!(uint32,   bench_enc_uint32,   bench_dec_uint32);
    bench_int_codec!(int32,    bench_enc_int32,    bench_dec_int32);
    bench_int_codec!(uint32_l, bench_enc_uint32_l, bench_dec_uint32_l);
    bench_int_codec!(int32_l,  bench_enc_int32_l,  bench_dec_int32_l);

    bench_int_codec!(uint64,   bench_enc_uint64,   bench_dec_uint64);
    bench_int_codec!(int64,    bench_enc_int64,    bench_dec_int64);
    bench_int_codec!(uint64_l, bench_enc_uint64_l, bench_dec_uint64_l);
    bench_int_codec!(int64_l,  bench_enc_int64_l,  bench_dec_int64_l);

    //
    // Ignore codec
    // 
    
    #[test]
    fn an_ignore_codec_should_round_trip() {
        assert_round_trip(ignore(4), &(), &Some(byte_vector!(0, 0, 0, 0)));
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
        assert_round_trip(constant(&input), &(), &Some(input));
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
        assert_round_trip(identity_bytes(), &input, &Some(input.clone()));
    }

    //
    // Bytes codec
    //

    #[test]
    fn a_byte_vector_codec_should_round_trip() {
        let input = byte_vector!(7, 1, 2, 3, 4);
        assert_round_trip(bytes(5), &input, &Some(input.clone()));
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
        assert_round_trip(codec, &7u8, &Some(byte_vector!(7)));
    }

    #[test]
    fn encoding_with_fixed_size_codec_should_pad_with_zeros_when_value_is_smaller_than_given_length() {
        let codec = fixed_size_bytes(3, uint8);
        assert_round_trip(codec, &7u8, &Some(byte_vector!(7, 0, 0)));
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
        let codec = variable_size_bytes(uint16, identity_bytes());
        assert_round_trip(codec, &input, &Some(byte_vector!(0, 5, 7, 1, 2, 3, 4)));
    }

    #[test]
    fn encoding_with_variable_size_codec_should_fail_when_length_of_encoded_value_is_too_large() {
        let input = byte_vector::fill(0x7, 256);
        let codec = variable_size_bytes(uint8, identity_bytes());
        assert_eq!(codec.encode(&input).unwrap_err().message(), "Length of encoded value (256 bytes) is greater than maximum value (255) of length type");
    }

    #[bench]
    fn bench_enc_variable_size_bytes(b: &mut Bencher) {
        let input = byte_vector!(7, 1, 2, 3, 4);
        let codec = variable_size_bytes(uint16, identity_bytes());
        b.iter(|| codec.encode(&input));
    }

    #[bench]
    fn bench_dec_variable_size_bytes(b: &mut Bencher) {
        let input = byte_vector!(0, 5, 7, 1, 2, 3, 4);
        let codec = variable_size_bytes(uint16, identity_bytes());
        b.iter(|| codec.decode(&input));
    }

    //
    // Eager bytes codec
    //

    #[test]
    fn an_eager_codec_should_round_trip() {
        let input = vec!(7, 1, 2, 3, 4);
        let codec = eager(variable_size_bytes(uint16, identity_bytes()));
        assert_round_trip(codec, &input, &Some(byte_vector!(0, 5, 7, 1, 2, 3, 4)));
    }

    //
    // Context injection ('|' operator)
    //
    
    #[allow(unused_parens)]
    #[test]
    fn context_should_be_pushed_when_using_the_bitor_operator() {
        // TODO: This test is temporarily written using with_context() rather than the `|` operator
        // while we figure out a solution for the operator overloading issues
        let input = byte_vector::empty();
        let codec = with_context("section", with_context("header", with_context("magic", uint8)));

        // Verify that the error message is prefexed with the correct context
        assert_eq!(codec.decode(&input).unwrap_err().message(), "section/header/magic: Requested read offset of 0 and length 1 bytes exceeds vector length of 0");
    }

    //
    // HList-related codecs
    //
    
    #[test]
    fn an_hnil_codec_should_round_trip() {
        assert_round_trip(hnil_codec(), &HNil, &Some(byte_vector::empty()));
    }

    #[test]
    fn an_hlist_prepend_codec_should_round_trip() {
        let codec1 = hlist_prepend_codec(uint8, hnil_codec());
        assert_round_trip(codec1, &hlist!(7u8), &Some(byte_vector!(7)));

        let codec2 = hlist_prepend_codec(uint8, hlist_prepend_codec(uint8, hnil_codec()));
        assert_round_trip(codec2, &hlist!(7u8, 3u8), &Some(byte_vector!(7, 3)));
    }

    #[test]
    fn an_hlist_flat_prepend_codec_should_round_trip() {
        let codec = hlist_flat_prepend_codec(uint8, |header| {
            Box::new(hcodec!({bytes((*header) as usize)} :: {uint16}))
        });
        assert_round_trip(codec, &hlist!(0x02u8, byte_vector!(0xAB, 0xCD), 0xCAFEu16), &Some(byte_vector!(0x02, 0xAB, 0xCD, 0xCA, 0xFE)));
    }

    #[test]
    fn an_hlist_codec_should_round_trip() {
        let codec = hcodec!({uint8} :: {uint8} :: {uint8}); 
        assert_round_trip(codec, &hlist!(7u8, 3u8, 1u8), &Some(byte_vector!(7, 3, 1)));
    }

    #[test]
    fn the_hcodec_macro_should_support_drop_left() {
        let c = byte_vector!(0xCA, 0xFE);
        let codec = hcodec!(
            { "magic"  => constant(&c) } >>
            { "field1" => uint8        } ::
            { "field2" => uint8        }
        );
        let bytes = byte_vector!(0xCA, 0xFE, 0x01, 0x02);
        let decoded = codec.decode(&bytes).unwrap().value;
        assert_eq!(decoded, hlist!(1, 2));
    }

    // This is implemented as a macro as otherwise we'd have to write out an explicit return type
    // and good luck with that...
    macro_rules! make_test_hcodec {
        {} => {
            {
                let m = byte_vector!(0xCA, 0xFE);
                hcodec!(
                    { "magic"      => constant(&m) } >>
                    { "version"    => uint8      } ::
                    { "junk_len"   => uint8      } >>= |junk_len| { Box::new(hcodec!(
                        { "skip"   => ignore(1)                  } >>
                        { "first"  => uint8                      } ::
                        { "junk"   => ignore(*junk_len as usize) } >>
                        { "second" => uint8                      } ::
                        { "third"  => uint8                      }
                        ))}
                )
            }
        };
    }

    #[test]
    fn the_hcodec_macro_should_work_with_a_mix_of_operations() {
        let codec = make_test_hcodec!();
        let input = hlist!(1u8, 3u8, 7u8, 3u8, 1u8);
        let expected = byte_vector!(0xCA, 0xFE, 0x01, 0x03, 0x00, 0x07, 0x00, 0x00, 0x00, 0x03, 0x01);
        assert_round_trip(codec, &input, &Some(expected));
    }

    #[bench]
    fn bench_enc_hlist(b: &mut Bencher) {
        let codec = make_test_hcodec!();
        let input = hlist!(1u8, 3u8, 7u8, 3u8, 1u8);
        b.iter(|| codec.encode(&input));
    }

    #[bench]
    fn bench_dec_hlist(b: &mut Bencher) {
        let codec = make_test_hcodec!();
        let input = byte_vector!(0xCA, 0xFE, 0x01, 0x03, 0x00, 0x07, 0x00, 0x00, 0x00, 0x03, 0x01);
        b.iter(|| codec.decode(&input));
    }
    
    //
    // Struct conversion codec
    //
    
    record_struct!(
        TestStruct1,
        foo: u8,
        bar: u8);

    #[test]
    fn record_structs_should_work() {
        let s1 = TestStruct1::from_hlist(hlist!(7u8, 3u8));
        assert_eq!(s1.foo, 7u8);
        assert_eq!(s1.bar, 3u8);
    }

    #[test]
    fn a_struct_codec_should_round_trip() {
        let codec = struct_codec!(TestStruct1 from {uint8} :: {uint8});
        assert_round_trip(codec, &TestStruct1 { foo: 7u8, bar: 3u8 }, &Some(byte_vector!(7, 3)));
    }

    //
    // Boxed codec and static ref support
    //

    #[test]
    fn boxed_codecs_should_work() {
        let codec = Box::new(struct_codec!(TestStruct1 from {uint8} :: {uint8}));
        assert_round_trip(codec, &TestStruct1 { foo: 7u8, bar: 3u8 }, &Some(byte_vector!(7, 3)));
    }

    const TEST_CODEC: &'static Codec<Value=i32> = &IntegralBECodec { _marker: PhantomData::<i32> };
    
    #[test]
    fn static_codecs_should_work() {
        assert_round_trip(TEST_CODEC, &0x12345678, &Some(byte_vector!(0x12, 0x34, 0x56, 0x78)));
    }
}
