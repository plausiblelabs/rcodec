//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//

#![feature(plugin)]
#![plugin(rcodec_macros)]

#[macro_use]
extern crate rcodec;

use std::fmt::Debug;
use rcodec::error::Error;
use rcodec::byte_vector::ByteVector;
use rcodec::codec::*;
use rcodec::hlist::*;

fn assert_round_trip_bytes<T: 'static + Eq + Debug, C: AsCodecRef<T>>(c: C, value: &T, raw_bytes: &Option<ByteVector>) {
    // Encode
    let codec = c.as_codec_ref();
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
    assert_round_trip_bytes(uint8, &7u8, &Some(byte_vector!(7)));
}

record_struct!(
    TestRecordVersion,
    compat_version: u8,
    feature_version: u8);

record_struct!(
    TestSectionRecord,
    offset: u8,
    length: u8);

record_struct!(
    TestFileHeader,
    version: TestRecordVersion,
    meta_section: TestSectionRecord,
    data_section: TestSectionRecord);

#[test]
fn a_complex_codec_should_round_trip() {
    let magic = byte_vector!(0xCA, 0xFE);
    
    let version_codec = struct_codec!(
        TestRecordVersion from
        { "compat_version"  | uint8  } ::
        { "feature_version" | uint8  } );

    let section_codec = || { struct_codec!(
        TestSectionRecord from
        { "section_offset"  | uint8  } ::
        { "section_length"  | uint8  } )
    };

    // TODO: Is there some way we can make header_codec use a shared reference to section_codec instead of resorting
    // to a closure that creates two copies of the section codec?
    let header_codec = struct_codec!(
        TestFileHeader from
        { "magic"           | constant(&magic) } >>
        { "file_version"    | version_codec    } ::
        { "meta_section"    | section_codec()  } ::
        { "data_section"    | section_codec()  } );

    let header = TestFileHeader {
        version: TestRecordVersion {
            compat_version: 1,
            feature_version: 2
        },
        meta_section: TestSectionRecord {
            offset: 0,
            length: 2
        },
        data_section: TestSectionRecord {
            offset: 2,
            length: 3
        }
    };
    
    assert_round_trip_bytes(header_codec, &header, &Some(
        byte_vector!(
            0xCA, 0xFE, // magic
            0x01, 0x02, // file_version
            0x00, 0x02, // meta_section
            0x02, 0x03  // data_section
                )));
}
