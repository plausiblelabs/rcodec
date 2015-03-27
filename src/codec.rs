//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

use std::rc::Rc;
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
pub struct DecoderResult<T> {
    pub value: T,
    pub remainder: ByteVector
}

/// A result type returned by Decoder operations.
pub type DecodeResult<T> = Result<DecoderResult<T>, Error>;

/// Unsigned 8-bit integer codec.
pub fn uint8() -> Codec<u8> {
    Codec {
        encoder: Box::new(|value| {
            // TODO: Use direct() once it's implemented
            Ok(byte_vector::buffered(&vec![*value]))
        }),
        decoder: Box::new(|bv| {
            // TODO: This is a dumbed down implementation just for evaluation purposes
            let v: &mut[u8] = &mut[0];
            match bv.read(v, 0, 1) {
                Ok(..) => {
                    match bv.drop(1) {
                        Ok(remainder) => Ok(DecoderResult { value: v[0], remainder: remainder }),
                        Err(e) => Err(e)
                    }
                },
                Err(e) => Err(e)
            }
        })
    }
}

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

/// Codec for HNil type.
#[allow(unused_variables)]
pub fn hnil_codec() -> Codec<HNil> {
    Codec {
        encoder: Box::new(|value| {
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
                        Err(Error::new("Encoded bytes do not match expected bytes".to_string()))
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
            Err(e) => panic!("Round-trip encoding failed: {:?}", e),
        }
    }

    #[test]
    fn a_u8_value_should_round_trip() {
        assert_round_trip_bytes(&uint8(), &7u8, &Some(byte_vector::buffered(&vec!(7u8))));
    }

    #[test]
    fn an_ignore_codec_should_round_trip() {
        assert_round_trip_bytes(&ignore(4), &(), &Some(byte_vector::buffered(&vec!(0u8, 0, 0, 0))));
    }

    #[test]
    fn decoding_with_ignore_codec_should_succeed_if_the_input_vector_is_long_enough() {
        let input = byte_vector::buffered(&vec!(7u8, 1, 2, 3, 4));
        let codec = ignore(3);
        match codec.decode(&input) {
            Ok(result) => {
                let expected_remainder = byte_vector::buffered(&vec!(3u8, 4));
                assert_eq!(expected_remainder, result.remainder);
            },
            Err(_) => assert!(false)
        }
    }

    #[test]
    fn decoding_with_ignore_codec_should_fail_if_the_input_vector_is_smaller_than_the_ignored_length() {
        let input = byte_vector::buffered(&vec!(1u8));
        let codec = ignore(3);
        match codec.decode(&input) {
            Ok(..) => assert!(false),
            Err(e) => assert_eq!(e.message(), "Requested length of 3 bytes exceeds vector length of 1".to_string())
        }
    }

    #[test]
    fn a_constant_codec_should_round_trip() {
        let input = byte_vector::buffered(&vec!(1u8, 2, 3, 4));
        assert_round_trip_bytes(&constant(&input), &(), &Some(input));
    }

    #[test]
    fn decoding_with_constant_codec_should_fail_if_the_input_vector_does_not_match_the_constant_vector() {
        let input = byte_vector::buffered(&vec!(1u8, 2, 3, 4));
        let codec = constant(&byte_vector::buffered(&vec!(6u8, 6, 6)));
        match codec.decode(&input) {
            Ok(..) => assert!(false),
            Err(e) => assert_eq!(e.message(), "Expected constant 060606 but got 010203".to_string())
        }
    }

    #[test]
    fn decoding_with_constant_codec_should_fail_if_the_input_vector_is_smaller_than_the_constant_vector() {
        let input = byte_vector::buffered(&vec!(1u8));
        let codec = constant(&byte_vector::buffered(&vec!(6u8, 6, 6)));
        match codec.decode(&input) {
            Ok(..) => assert!(false),
            Err(e) => assert_eq!(e.message(), "Requested view offset of 0 and length 3 bytes exceeds vector length of 1".to_string())
        }
    }

    #[test]
    fn an_hnil_should_round_trip() {
        assert_round_trip_bytes(&hnil_codec(), &HNil, &Some(byte_vector::empty()));
    }

    #[test]
    fn an_hlist_prepend_codec_should_work() {
        let codec1 = hlist_prepend_codec(uint8(), hnil_codec());
        assert_round_trip_bytes(&codec1, &hlist!(7u8), &Some(byte_vector::buffered(&vec!(7u8))));

        let codec2 = hlist_prepend_codec(uint8(), codec1);
        assert_round_trip_bytes(&codec2, &hlist!(7u8, 3u8), &Some(byte_vector::buffered(&vec!(7u8, 3u8))));
    }

    #[test]
    fn an_hlist_codec_should_round_trip() {
        let codec = hcodec!(uint8(), uint8(), uint8()); 
        assert_round_trip_bytes(&codec, &hlist!(7u8, 3u8, 1u8), &Some(byte_vector::buffered(&vec!(7u8, 3u8, 1u8))));
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
        match codec.decode(&input) {
            Ok(..) => assert!(false),
            Err(e) => assert_eq!(e.message(), "section/header/magic: Requested read offset of 0 and length 1 bytes exceeds vector length of 0")
        }
    }

    #[test]
    fn the_hcodec_macro_should_work_with_context_injected_codecs() {
        let codec = hcodec!(
            ("first"  | uint8()),
            ("second" | uint8()),
            ("third"  | uint8()));
        assert_round_trip_bytes(&codec, &hlist!(7u8, 3u8, 1u8), &Some(byte_vector::buffered(&vec!(7u8, 3u8, 1u8))));
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
        let codec = scodec!(TestStruct2, hcodec!(uint8(), uint8()));
        assert_round_trip_bytes(&codec, &TestStruct2 { foo: 7u8, bar: 3u8 }, &Some(byte_vector::buffered(&vec!(7u8, 3u8))));
    }
}
