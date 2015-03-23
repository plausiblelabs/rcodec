//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

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
    fn encode(&self, value: &T) -> EncodeResult {
        (*self.encoder)(value)
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<T> {
        (*self.decoder)(bv)
    }
}

/// A result type returned by Encoder operations.
type EncodeResult = Result<ByteVector, Error>;

/// A result type, consisting of a decoded value and any unconsumed data, returned by Decoder operations.
#[allow(dead_code)]
pub struct DecoderResult<T> {
    value: T,
    remainder: ByteVector
}

/// A result type returned by Decoder operations.
type DecodeResult<T> = Result<DecoderResult<T>, Error>;

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

/// XXX: Rough sketch of an HList codec
// struct HListPrependEncoder<A, L: HList> { a: Box<Codec<A>>, l: Box<Codec<L>> }
// impl<A, L: HList> Encoder<HCons<A, L>> for HListPrependEncoder<A, L> {
//     fn encode(&self, value: &HCons<A, L>) -> EncodeResult {
//         Err(Error { description: "Not yet implemented".to_string() })
//     }
// }

// struct HListPrependDecoder<A, L: HList> { a: Box<Codec<A>>, l: Box<Codec<L>> }
// impl<A, L: HList> Decoder<HCons<A, L>> for HListPrependDecoder<A, L> {
//     fn decode(&self, bv: &ByteVector) -> DecodeResult<HCons<A, L>> {
//         Err(Error { description: "Not yet implemented".to_string() })
//     }
// }

// pub fn hlist_prepend_codec<A, L: HList>(a: Codec<A>, l: Codec<L>) -> Codec<HCons<A, L>> {
//     Codec {
//         encoder: Box::new(HListPrependEncoder { a: Box::new(a), l: Box::new(l) }),
//         decoder: Box::new(HListPrependDecoder { a: Box::new(a), l: Box::new(l) })
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

    fn assert_round_trip_bytes<T: Eq + Debug>(codec: Codec<T>, value: &T, raw_bytes: Option<ByteVector>) {
        // Encode
        let result = codec.encode(value).and_then(|encoded| {
            // Compare encoded bytes to the expected bytes, if provided
            let compare_result = match raw_bytes {
                Some(ref expected) => {
                    if encoded != *expected {
                        Err(Error { description: "Encoded bytes do not match expected bytes".to_string() })
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
        assert_round_trip_bytes(uint8(), &7u8, Some(byte_vector::buffered(&vec!(7u8))));
    }

    #[test]
    fn an_hnil_should_round_trip() {
        assert_round_trip_bytes(hnil_codec(), &HNil, Some(byte_vector::empty()));
    }

    // #[test]
    // fn an_hlist_prepend_codec_should_work() {
    //     let codec1 = hlist_prepend_codec(uint8(), hnil_codec());
    //     assert_round_trip_bytes(codec1, &hlist!(7u8), Some(byte_vector::buffered(&vec!(7u8))));

    //     // let codec2 = hlist_prepend_codec(uint8(), codec1);
    //     // assert_round_trip_bytes(codec2, &hlist!(7u8, 3u8), Some(byte_vector::buffered(&vec!(7u8, 3u8))));
    // }
    
    // #[test]
    // fn an_hlist_codec_should_round_trip() {
    //     let codec = hcodec!(uint8(), uint8());
    //     assert_round_trip_bytes(codec, &Box::new(hlist!(7u8, 3u8)), Some(byte_vector::buffered(&vec!(7u8, 3u8))));
    // }
}
