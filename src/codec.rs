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

/// Implements encoding and decoding of values of type `T`.
#[allow(dead_code)]
pub struct Codec<T> {
    encoder: Box<Encoder<T>>,
    decoder: Box<Decoder<T>>
}

#[allow(dead_code)]
impl<T> Codec<T> {
    fn encode(&self, value: &T) -> EncodeResult {
        self.encoder.encode(value)
    }

    fn decode(&self, bv: &ByteVector) -> DecodeResult<T> {
        self.decoder.decode(bv)
    }
}

/// A result type returned by Encoder operations.
type EncodeResult = Result<ByteVector, Error>;

/// Implements encoding a value of type `T` to a ByteVector.
trait Encoder<T> {
    fn encode(&self, value: &T) -> EncodeResult;
}

/// A result type, consisting of a decoded value and any unconsumed data, returned by Decoder operations.
#[allow(dead_code)]
pub struct DecoderResult<T> {
    value: T,
    remainder: ByteVector
}

/// A result type returned by Decoder operations.
type DecodeResult<T> = Result<DecoderResult<T>, Error>;

/// Implements decoding a value of type `T` from a ByteVector.
trait Decoder<T> {
    fn decode(&self, bv: &ByteVector) -> DecodeResult<T>;
}

/// TODO
struct IntegralEncoder;
impl Encoder<u8> for IntegralEncoder {
    fn encode(&self, value: &u8) -> EncodeResult {
        // TODO: Use direct() once it's implemented
        Ok(byte_vector::buffered(&vec![*value]))
    }
}

struct IntegralDecoder;
impl Decoder<u8> for IntegralDecoder {
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
pub fn uint8() -> Codec<u8> {
    Codec {
        encoder: Box::new(IntegralEncoder),
        decoder: Box::new(IntegralDecoder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use error::Error;
    use byte_vector;
    use byte_vector::ByteVector;

    fn assert_round_trip_bytes(codec: Codec<u8>, value: &u8, raw_bytes: Option<ByteVector>) {
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
}
