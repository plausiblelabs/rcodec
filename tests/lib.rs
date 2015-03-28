//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//

#[macro_use]
extern crate rcodec;

use std::fmt::Debug;
use rcodec::error::Error;
use rcodec::byte_vector::ByteVector;
use rcodec::codec::*;

fn assert_round_trip_bytes<T: Eq + Debug>(codec: Box<Codec<T>>, value: &T, raw_bytes: &Option<ByteVector>) {
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
    assert_round_trip_bytes(uint8(), &7u8, &Some(byte_vector!(7)));
}
