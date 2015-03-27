//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

use std::mem::size_of;
use std::num::Int;
use std::num::FromPrimitive;
use std::num;
use std::rc::Rc;
use std::vec;
use core;

use error::Error;
use byte_vector;
use byte_vector::ByteVector;
use hlist::*;

/// Implements encoding and decoding of values of type `T`.
#[allow(dead_code)]
pub struct Codec<T> {
    encoder: Box<Fn(&T) -> EncodeResult>,
    decoder: Box<Fn(&ByteVector) -> DecodeResult<T>>
}

#[allow(dead_code)]
impl<T> Codec<T> {
    pub fn encode(&self, value: &T) -> EncodeResult {
        (*self.encoder)(value)
    }

    pub fn decode(&self, bv: &ByteVector) -> DecodeResult<T> {
        (*self.decoder)(bv)
    }
}

/// A result type returned by Encoder operations.
pub type EncodeResult = Result<ByteVector, Error>;

/// A result type, consisting of a decoded value and any unconsumed data, returned by Decoder operations.
#[allow(dead_code)]
#[derive(Debug)]
pub struct DecoderResult<T> {
    pub value: T,
    pub remainder: ByteVector
}

/// A result type returned by Decoder operations.
pub type DecodeResult<T> = Result<DecoderResult<T>, Error>;

/// Generic unsigned integer codec.
pub fn uint<T: Int + FromPrimitive>() -> Codec<T> {
    Codec {
        encoder: Box::new(|value: &T| {
            // TODO: Use direct() once it's implemented
            let size = size_of::<T>();
            let mut v = Vec::<u8>::with_capacity(size);
            for i in 0..size {
                let shift = (size - i - 1) * 8;
                let byte = (*value >> shift) & num::FromPrimitive::from_u8(0xff).unwrap();
                v.push(num::cast(byte).unwrap());
            }
            Ok(byte_vector::buffered(&v))
        }),
        decoder: Box::new(|bv| {
            let size = size_of::<T>();
            let v = &mut vec::from_elem(0u8, size);
            match bv.read(v, 0, size) {
                Ok(..) => {
                    let mut value = T::zero();
                    for i in 0..size {
                        value = (value << 8) | num::FromPrimitive::from_u8(v[i]).unwrap()
                    }
                    match bv.drop(size) {
                        Ok(remainder) => Ok(DecoderResult { value: value, remainder: remainder }),
                        Err(e) => Err(e)
                    }
                },
                Err(e) => Err(e)
            }
        })
    }
}

/// Unsigned 8-bit integer codec.
pub fn uint8() -> Codec<u8> { uint() }

/// Unsigned 16-bit integer codec.
pub fn uint16() -> Codec<u16> { uint() }

/// Unsigned 32-bit integer codec.
pub fn uint32() -> Codec<u32> { uint() }

/// Unsigned 64-bit integer coder.
pub fn uint64() -> Codec<u64> { uint() }

/// Codec that encodes `len` low bytes and decodes by discarding `len` bytes.
pub fn ignore(len: usize) -> Codec<()> {
    // TODO: Is there a better way?
    let encode_len = len.clone();
    let decode_len = len.clone();
    
    Codec {
        encoder: Box::new(move |_unit| {
            Ok(byte_vector::fill(0, encode_len))
        }),
        decoder: Box::new(move |bv| {
            bv.drop(decode_len).map(|remainder| {
                DecoderResult { value: (), remainder: remainder }
            })
        })
    }
}

/// Codec that always encodes the given byte vector, and decodes by returning a unit result if the actual bytes match
/// the given byte vector or an error otherwise.
pub fn constant(bytes: &ByteVector) -> Codec<()> {
    // TODO: Can we avoid all the extra cloning here?
    let encoder_bytes = (*bytes).clone();
    let decoder_bytes = (*bytes).clone();
    
    Codec {
        encoder: Box::new(move |_unit| {
            Ok(encoder_bytes.clone())
        }),
        decoder: Box::new(move |bv| {
            bv.take(decoder_bytes.length()).and_then(|taken| {
                if taken == decoder_bytes {
                    Ok(DecoderResult { value: (), remainder: bv.drop(decoder_bytes.length()).unwrap() })
                } else {
                    Err(Error::new(format!("Expected constant {:?} but got {:?}", decoder_bytes, taken)))
                }
            })
        })
    }
}

/// Identity byte vector codec.
///   - Encodes by returning the given byte vector.
///   - Decodes by taking all remaining bytes from the given byte vector.
pub fn identity_bytes() -> Codec<ByteVector> {
    Codec {
        encoder: Box::new(|value: &ByteVector| {
            Ok((*value).clone())
        }),
        decoder: Box::new(|bv| {
            Ok(DecoderResult { value: (*bv).clone(), remainder: byte_vector::empty() })
        })
    }
}

/// Byte vector codec.
///   - Encodes by returning the given byte vector if its length is `len` bytes, otherwise returns an error.
///   - Decodes by taking `len` bytes from the given byte vector.
pub fn bytes(len: usize) -> Codec<ByteVector> {
    fixed_size_bytes(len, identity_bytes())
}

/// Codec that limits the number of bytes that are available to `codec`.
///
/// When encoding, if the given `codec` encodes fewer than `len` bytes, the byte vector
/// is right padded with low bytes.  If `codec` instead encodes more than `len` bytes,
/// an error is returned.
///
/// When decoding, the given `codec` is only given `len` bytes.  If `codec` does
/// not consume all `len` bytes, any remaining bytes are discarded.
pub fn fixed_size_bytes<T: 'static>(len: usize, codec: Codec<T>) -> Codec<T> {
    // XXX: Ugh
    let encoder = Rc::new(codec);
    let decoder = encoder.clone();
    let encoder_len = len.clone();
    let decoder_len = len.clone();

    Codec {
        encoder: Box::new(move |value| {
            encoder.encode(value).and_then(|encoded| {
                if encoded.length() > encoder_len {
                    Err(Error::new(format!("Encoding requires {} bytes but codec is limited to fixed length of {}", encoded.length(), encoder_len)))
                } else {
                    encoded.pad_right(encoder_len)
                }
            })
        }),
        decoder: Box::new(move |bv| {
            // Give `len` bytes to the decoder; if successful, return the result along with
            // the remainder of `bv` after dropping `len` bytes from it
            bv.take(decoder_len).and_then(|taken| {
                decoder.decode(&taken).map(|decoded| {
                    DecoderResult { value: decoded.value, remainder: bv.drop(decoder_len).unwrap() }
                })
            })
        })
    }
}

/// Codec for HNil type.
pub fn hnil_codec() -> Codec<HNil> {
    Codec {
        encoder: Box::new(|_hnil| {
            Ok(byte_vector::empty())
        }),
        decoder: Box::new(|bv| {
            Ok(DecoderResult { value: HNil, remainder: bv.clone() })
        })
    }
}

/// Codec used to convert an HList of codecs into a single codec that encodes/decodes an HList of values.
pub fn hlist_prepend_codec<A: 'static, L: 'static + HList>(a_codec: Codec<A>, l_codec: Codec<L>) -> Codec<HCons<A, L>> {
    // XXX: Holy moly. This is my attempt at making it possible to capture the codecs in the two separate closures below.
    let a_encoder = Rc::new(a_codec);
    let a_decoder = a_encoder.clone();
    let l_encoder = Rc::new(l_codec);
    let l_decoder = l_encoder.clone();
    
    Codec {
        encoder: Box::new(move |value: &HCons<A, L>| {
            // TODO: Generalize this as an encode_both() function
            a_encoder.encode(&value.0).and_then(|encoded_a| {
                l_encoder.encode(&value.1).map(|encoded_l| byte_vector::append(&encoded_a, &encoded_l))
            })
        }),
        decoder: Box::new(move |bv| {
            // TODO: Generalize this as a decode_both_combine() function
            a_decoder.decode(&bv).and_then(|decoded_a| {
                l_decoder.decode(&decoded_a.remainder).map(move |decoded_l| {
                    DecoderResult { value: HCons(decoded_a.value, decoded_l.value), remainder: decoded_l.remainder }
                })
            })
        })
    }
}

/// Override for the '|' operator that creates a new codec that injects additional context (e.g. in error messages)
/// into the codec on the right-hand side.
impl<T: 'static> core::ops::BitOr<Codec<T>> for &'static str {
    type Output = Codec<T>;

    fn bitor(self, rhs: Codec<T>) -> Codec<T> {
        let encoder = Rc::new(rhs);
        let decoder = encoder.clone();

        // XXX: Ugh
        let encoder_ctx = self.clone();
        let decoder_ctx = self.clone();

        Codec {
            encoder: Box::new(move |value| {
                encoder.encode(value).map_err(|e| e.push_context(encoder_ctx))
            }),
            decoder: Box::new(move |bv| {
                decoder.decode(bv).map_err(|e| e.push_context(decoder_ctx))
            })
        }
    }
}

/// Returns a new codec that encodes/decodes the unit value followed by the right-hand value,
/// discarding the unit value when decoding.
pub fn drop_left<T: 'static>(lhs: Codec<()>, rhs: Codec<T>) -> Codec<T> {
    // XXX: Ugh
    let lhs_encoder = Rc::new(lhs);
    let lhs_decoder = lhs_encoder.clone();
    
    let rhs_encoder = Rc::new(rhs);
    let rhs_decoder = rhs_encoder.clone();

    // TODO: Add generic xmap function
    Codec {
        encoder: Box::new(move |value: &T| {
            lhs_encoder.encode(&()).and_then(|encoded_lhs| {
                rhs_encoder.encode(value).map(|encoded_rhs| {
                    byte_vector::append(&encoded_lhs, &encoded_rhs)
                })
            })
        }),
        decoder: Box::new(move |bv| {
            lhs_decoder.decode(bv).and_then(|decoded| {
                rhs_decoder.decode(&decoded.remainder)
            })
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

    fn assert_round_trip_bytes<T: Eq + Debug>(codec: &Codec<T>, value: &T, raw_bytes: &Option<ByteVector>) {
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

    #[test]
    fn a_u8_value_should_round_trip() {
        assert_round_trip_bytes(&uint8(), &7u8, &Some(byte_vector!(7)));
    }
    
    #[test]
    fn a_u16_value_should_round_trip() {
        assert_round_trip_bytes(&uint16(), &0x1234u16, &Some(byte_vector!(0x12, 0x34)));
    }

    #[test]
    fn a_u32_value_should_round_trip() {
        assert_round_trip_bytes(&uint32(), &0x12345678u32, &Some(byte_vector!(0x12, 0x34, 0x56, 0x78)));
    }

    #[test]
    fn a_u64_value_should_round_trip() {
        assert_round_trip_bytes(&uint64(), &0x1234567890abcdef, &Some(byte_vector!(0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef)));
    }

    #[test]
    fn an_ignore_codec_should_round_trip() {
        assert_round_trip_bytes(&ignore(4), &(), &Some(byte_vector!(0, 0, 0, 0)));
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

    #[test]
    fn a_constant_codec_should_round_trip() {
        let input = byte_vector!(1, 2, 3, 4);
        assert_round_trip_bytes(&constant(&input), &(), &Some(input));
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

    #[test]
    fn an_identity_codec_should_round_trip() {
        let input = byte_vector!(1, 2, 3, 4);
        assert_round_trip_bytes(&identity_bytes(), &input, &Some(input.clone()));
    }

    #[test]
    fn a_byte_vector_codec_should_round_trip() {
        let input = byte_vector!(7, 1, 2, 3, 4);
        assert_round_trip_bytes(&bytes(5), &input, &Some(input.clone()));
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

    #[test]
    fn a_fixed_size_bytes_codec_should_round_trip() {
        let codec = fixed_size_bytes(1, uint8());
        assert_round_trip_bytes(&codec, &7u8, &Some(byte_vector!(7)));
    }

    #[test]
    fn encoding_with_fixed_size_codec_should_pad_with_zeros_when_value_is_smaller_than_given_length() {
        let codec = fixed_size_bytes(3, uint8());
        assert_round_trip_bytes(&codec, &7u8, &Some(byte_vector!(7, 0, 0)));
    }

    #[test]
    fn encoding_with_fixed_size_codec_should_fail_when_value_needs_more_space_than_given_length() {
        let codec = fixed_size_bytes(1, constant(&byte_vector!(6, 6, 6)));
        assert_eq!(codec.encode(&()).unwrap_err().message(), "Encoding requires 3 bytes but codec is limited to fixed length of 1");
    }

    #[test]
    fn decoding_with_fixed_size_codec_should_return_remainder_that_had_len_bytes_dropped() {
        let input = byte_vector!(7, 1, 2, 3, 4);
        let codec = fixed_size_bytes(3, uint8());
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

    #[test]
    fn an_hnil_codec_should_round_trip() {
        assert_round_trip_bytes(&hnil_codec(), &HNil, &Some(byte_vector::empty()));
    }

    #[test]
    fn an_hlist_prepend_codec_should_round_trip() {
        let codec1 = hlist_prepend_codec(uint8(), hnil_codec());
        assert_round_trip_bytes(&codec1, &hlist!(7u8), &Some(byte_vector!(7)));

        let codec2 = hlist_prepend_codec(uint8(), codec1);
        assert_round_trip_bytes(&codec2, &hlist!(7u8, 3u8), &Some(byte_vector!(7, 3)));
    }

    #[test]
    fn an_hlist_codec_should_round_trip() {
        let codec = hcodec!({uint8()} :: {uint8()} :: {uint8()}); 
        assert_round_trip_bytes(&codec, &hlist!(7u8, 3u8, 1u8), &Some(byte_vector!(7, 3, 1)));
    }

    #[allow(unused_parens)]
    #[test]
    fn context_should_be_pushed_when_using_the_bitor_operator() {
        let input = byte_vector::empty();
        let codec =
            ("section" |
             ("header" |
              ("magic" | uint8())
              )
             );

        // Verify that the error message is prefexed with the correct context
        assert_eq!(codec.decode(&input).unwrap_err().message(), "section/header/magic: Requested read offset of 0 and length 1 bytes exceeds vector length of 0");
    }

    #[test]
    fn the_hcodec_macro_should_work_with_context_injected_codecs() {
        let m = byte_vector!(0xCA, 0xFE);
        let codec = hcodec!(
            { "magic"  | constant(&m) } >>
            { "first"  | uint8()      } ::
            { "trash"  | ignore(1)    } >>
            { "second" | uint8()      } :: 
            { "third"  | uint8()      }
        );
        
        let input = hlist!(7u8, 3u8, 1u8);
        let expected = byte_vector!(0xCA, 0xFE, 0x07, 0x00, 0x03, 0x01);
        assert_round_trip_bytes(&codec, &input, &Some(expected));
    }

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
        let codec = scodec!(TestStruct2, hcodec!({uint8()} :: {uint8()}));
        assert_round_trip_bytes(&codec, &TestStruct2 { foo: 7u8, bar: 3u8 }, &Some(byte_vector!(7, 3)));
    }
}
