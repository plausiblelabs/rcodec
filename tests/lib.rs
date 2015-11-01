//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//

#![feature(plugin, custom_attribute)]
#![plugin(rcodec_macros)]

#[macro_use]
extern crate rcodec;

use std::fmt::Debug;
use rcodec::error::Error;
use rcodec::byte_vector::ByteVector;
use rcodec::codec::*;
use rcodec::hlist::*;

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

#[test]
fn a_u8_value_should_round_trip() {
    assert_round_trip(uint8(), &7u8, &Some(byte_vector!(7)));
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

record_struct!(
    TestFileItem,
    header: TestFileHeader,
    metadata: Vec<u8>,
    data: Vec<u8>);

#[test]
fn a_complex_codec_should_round_trip() {
    const FILE_HEADER_SIZE: u8 = 6;
    
    let magic = byte_vector!(0xCA, 0xFE);

    let version_codec = struct_codec!(
        TestRecordVersion from
        { "compat_version"  => uint8()  } ::
        { "feature_version" => uint8()  } );

    let section_codec = || { Box::new(struct_codec!(
        TestSectionRecord from
        { "section_offset"  => uint8()  } ::
        { "section_length"  => uint8()  } ))
    };

    let header_codec = struct_codec!(
        TestFileHeader from
        { "magic"           => constant(&magic)      } >>
        { "file_version"    => version_codec         } ::
        { "meta_section"    => *section_codec()      } ::
        { "data_section"    => *section_codec()      } );

    let item_codec = struct_codec!(
        TestFileItem from
        { "header"          => header_codec } >>= |hdr| {
            let padding_1_len = (hdr.meta_section.offset - FILE_HEADER_SIZE) as usize;
            let metadata_len  = hdr.meta_section.length as usize;
            let padding_2_len = (hdr.data_section.offset as usize) - ((hdr.meta_section.offset as usize) + metadata_len);
            let data_len      = hdr.data_section.length as usize;
            Box::new(hcodec!(
                { "padding_1"   => ignore(padding_1_len)      } >>
                { "metadata"    => eager(bytes(metadata_len)) } ::
                { "padding_2"   => ignore(padding_2_len)      } >>
                { "data"        => eager(bytes(data_len))     } ))
        });
    
    let header = TestFileHeader {
        version: TestRecordVersion {
            compat_version: 1,
            feature_version: 2
        },
        meta_section: TestSectionRecord {
            offset: FILE_HEADER_SIZE + 2,
            length: 2
        },
        data_section: TestSectionRecord {
            offset: FILE_HEADER_SIZE + 6,
            length: 2
        }
    };

    let item = TestFileItem {
        header: header,
        metadata: vec!(1, 7),
        data: vec!(6, 6)
    };
    
    assert_round_trip(item_codec, &item, &Some(
        byte_vector!(
            0xCA, 0xFE, // magic
            0x01, 0x02, // file_version
            0x08, 0x02, // meta_section
            0x0C, 0x02, // data_section
            0x00, 0x00, // padding_1 (ignored)
            0x01, 0x07, // metadata
            0x00, 0x00, // padding_2 (ignored)
            0x06, 0x06  // data
                )));
}
