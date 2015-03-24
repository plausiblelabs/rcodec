//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

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

/// A result type returned by encode operations.
pub type EncodeResult = Result<ByteVector, Error>;

/// A result type, consisting of a decoded value and any unconsumed data, returned by decode operations.
#[allow(dead_code)]
pub struct DecoderResult<T> {
    value: T,
    remainder: ByteVector
}

/// A result type returned by decode operations.
pub type DecodeResult<T> = Result<DecoderResult<T>, Error>;

/// Codec that operates on integral types.
pub struct IntegralCodec;
impl Codec<u8> for IntegralCodec {
    fn encode(&self, value: &u8) -> EncodeResult {
        // TODO: Use direct() once it's implemented
        Ok(byte_vector::buffered(&vec![*value]))
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<u8> {
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
    }
}

/// Unsigned 8-bit integer codec.
pub static uint8: IntegralCodec = IntegralCodec;

/// Codec for HNil type.
pub struct HNilCodec;
impl Codec<HNil> for HNilCodec {
    #[allow(unused_variables)]
    fn encode(&self, value: &HNil) -> EncodeResult {
        Ok(byte_vector::empty())
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<HNil> {
        // TODO: Can we avoid creating a view here?  Or at least make ByteVector implement Clone?
        bv.drop(0).map(|rem| DecoderResult { value: HNil, remainder: rem })
    }
}
pub static hnil_codec: HNilCodec = HNilCodec;
    
/// Codec used to convert an HList of codecs into a single codec that encodes/decodes an HList of values.
pub struct HListPrependCodec<'a, A, L: HList>(&'a Codec<A>, &'a Codec<L>);
impl<'a, A, L: HList> Codec<HCons<A, L>> for HListPrependCodec<'a, A, L> {
    fn encode(&self, value: &HCons<A, L>) -> EncodeResult {
        // TODO: Generalize this as an encode_both() function
        self.0.encode(&value.head()).and_then(|encoded_a| {
            self.1.encode(&value.tail()).map(|encoded_l| byte_vector::append(&encoded_a, &encoded_l))
        })
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<HCons<A, L>> {
        // TODO: Generalize this as a decode_both_combine() function
        self.0.decode(&bv).and_then(|decoded_a| {
            self.1.decode(&decoded_a.remainder).map(move |decoded_l| {
                DecoderResult { value: HCons(decoded_a.value, decoded_l.value), remainder: decoded_l.remainder }
            })
        })
    }
}

// /// Codec that injects additional context into another codec.
// struct ContextCodec<'a, T> { codec: &'a Codec<T>, ctx: &'static str }
// impl<'a, T> Codec<T> for ContextCodec<'a, T> {
//     fn encode(&self, value: &T) -> EncodeResult {
//         self.codec.encode(value).map_err(|e| e.push_context(self.ctx))
//     }

//     fn decode(&self, bv: &ByteVector) -> DecodeResult<T> {
//         self.codec.decode(bv).map_err(|e| e.push_context(self.ctx))
//     }
// }

/// Override for the '|' operator that creates a new codec that injects additional context (e.g. for error messages)
/// into the codec on the right-hand side.
// impl<T: 'static> core::ops::BitOr<Codec<T>> for &'static str {
//     type Output = Codec<T>;

//     fn bitor(self, rhs: Codec<T>) -> Codec<T> {
//         ContextCodec { codec: &rhs, ctx: self }
//     }
// }

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
        assert_round_trip_bytes(&uint8, &7u8, &Some(byte_vector::buffered(&vec!(7u8))));
    }

    #[test]
    fn an_hnil_should_round_trip() {
        assert_round_trip_bytes(&hnil_codec, &HNil, &Some(byte_vector::empty()));
    }

    #[test]
    fn an_hlist_prepend_codec_should_work() {
        let codec1 = HListPrependCodec(&uint8, &hnil_codec);
        assert_round_trip_bytes(&codec1, &hlist!(7u8), &Some(byte_vector::buffered(&vec!(7u8))));

        let codec2 = HListPrependCodec(&uint8, &codec1);
        assert_round_trip_bytes(&codec2, &hlist!(7u8, 3u8), &Some(byte_vector::buffered(&vec!(7u8, 3u8))));
    }

    // #[test]
    // fn an_hlist_codec_should_round_trip() {
    //     let codec = hcodec!(&uint8, &uint8, &uint8);
    //     assert_round_trip_bytes(&codec, &hlist!(7u8, 3u8, 1u8), &Some(byte_vector::buffered(&vec!(7u8, 3u8, 1u8))));
    // }

    // #[allow(unused_parens)]
    // #[test]
    // fn context_should_be_pushed_when_using_the_bitor_operator() {
    //     let input = byte_vector::empty();
    //     let codec =
    //         ("section" |
    //          ("header" |
    //           ("magic" | uint8())
    //           )
    //          );

    //     // Verify that the error message is prefexed with the correct context
    //     match codec.decode(&input) {
    //         Ok(..) => assert!(false),
    //         Err(e) => assert_eq!(e.message(), "section/header/magic: Requested read offset of 0 and length 1 bytes exceeds vector length of 0")
    //     }
    // }

    // #[test]
    // fn the_hcodec_macro_should_work_with_context_injected_codecs() {
    //     let codec = hcodec!(
    //         ("first"  | uint8()),
    //         ("second" | uint8()),
    //         ("third"  | uint8()));
    //     assert_round_trip_bytes(&codec, &hlist!(7u8, 3u8, 1u8), &Some(byte_vector::buffered(&vec!(7u8, 3u8, 1u8))));
    // }
}
