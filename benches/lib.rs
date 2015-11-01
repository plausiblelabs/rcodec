//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//

#![feature(plugin, custom_attribute, test)]
#![plugin(rcodec_macros)]

#[macro_use]
extern crate rcodec;

extern crate test;

use test::Bencher;
use rcodec::codec::*;
use rcodec::hlist::*;

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

const FILE_HEADER_SIZE: u8 = 6;
    
macro_rules! make_complex_codec {
    {} => {
        {
            let magic = byte_vector!(0xCA, 0xFE);

            let version_codec = struct_codec!(
                TestRecordVersion from
                { "compat_version"  => uint8  } ::
                { "feature_version" => uint8  } );

            let section_codec = || { Box::new(struct_codec!(
                TestSectionRecord from
                { "section_offset"  => uint8  } ::
                { "section_length"  => uint8  } ))
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

            item_codec
        }
    };
}

fn make_test_file_item() -> TestFileItem {
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

    TestFileItem {
        header: header,
        metadata: vec!(1, 7),
        data: vec!(6, 6)
    }
}

#[bench]
fn bench_enc_complex_item(b: &mut Bencher) {
    let codec = make_complex_codec!();
    let input = make_test_file_item();
    b.iter(|| codec.encode(&input));
}

#[bench]
fn bench_dec_complex_item(b: &mut Bencher) {
    let codec = make_complex_codec!();
    let input = byte_vector!(
        0xCA, 0xFE, // magic
        0x01, 0x02, // file_version
        0x08, 0x02, // meta_section
        0x0C, 0x02, // data_section
        0x00, 0x00, // padding_1 (ignored)
        0x01, 0x07, // metadata
        0x00, 0x00, // padding_2 (ignored)
        0x06, 0x06  // data
    );
    b.iter(|| codec.decode(&input));
}
