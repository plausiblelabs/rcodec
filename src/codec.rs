//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

#![plugin(record_struct)]

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

/// Codec for HNil type.
#[allow(unused_variables)]
pub fn hnil_codec() -> Codec<HNil> {
    Codec {
        encoder: Box::new(|value| {
            Ok(byte_vector::empty())
        }),
        decoder: Box::new(|bv| {
            // TODO: Can we avoid creating a view here?  Or at least make ByteVector implement Clone?
            bv.drop(0).map(|rem| DecoderResult { value: HNil, remainder: rem })
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

    // record_struct!(
    //     TestStruct, HCons<u8, HCons<u8, HNil>>,
    //     foo: u8,
    //     bar: u8);

    #[test]
    fn a_struct_codec_should_round_trip() {
        // let codec = scodec!(TestStruct, hcodec!(uint8(), uint8()));
        // assert_round_trip_bytes(&codec, &TestStruct { foo: 7u8, bar: 3u8 }, &Some(byte_vector::buffered(&vec!(7u8, 3u8))));
    }
}
