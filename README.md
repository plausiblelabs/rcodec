# rcodec

This Rust library provides combinators for purely functional, declarative encoding and decoding of binary data.  Its design is largely derived from that of the [scodec](https://github.com/scodec/scodec) library for Scala.

## Usage

Add the following dependencies to your `Cargo.toml`:

```toml
[dependencies]
rcodec = { git = "https://opensource.plausible.coop/src/scm/rc/rcodec.git" }
hlist = { git = "https://opensource.plausible.coop/src/scm/rc/hlist-rs.git" }
hlist_macros = { git = "https://opensource.plausible.coop/src/scm/rc/hlist-rs.git" }
```

Then, in your crate:

```rust
// The following allows for using custom `HListSupport` attribute defined in hlist_macros crate.
#![feature(plugin, custom_attribute)]
#![plugin(hlist_macros)]

#[macro_use]
extern crate hlist;

#[macro_use]
extern crate rcodec;

use rcodec::byte_vector::ByteVector;
use rcodec::codec::*;
use hlist::*;
```

## Examples

The codec module provides a number of predefined codecs.  In the following example, we use the `uint32` codec to encode a `u32` value to a `ByteVector` representation, and then decode the `ByteVector` back to its `u32` representation:

```rust
let codec = uint32;
let v0 = 258u32;
let bv = codec.encode(v0).unwrap();
assert_eq(bv, byte_vector!(0x00, 0x00, 0x01, 0x02));
let v1 = codec.decode(bv).unwrap().value;
assert_eq(v0, v1);
```

Automatic binding to structs when encoding/decoding is supported via the [hlist](https://opensource.plausible.coop/src/scm/rc/hlist-rs.git) crate:

```rust
#[derive(Debug, PartialEq, Eq, Clone)]
#[HListSupport]
struct TestStruct {
    foo: u8,
    bar: u16
}

let codec = struct_codec!(TestStruct from {uint8} :: {uint16});
let s0 = TestStruct { foo: 7u8, bar: 3u16 };
let bv = codec.encode(&s0).unwrap();
assert_eq(bv, byte_vector!(7, 0, 3));
let s1 = codec.decode(&bv).unwrap().value;
assert_eq(s0, s1);
```

Here's an example of a more complex codec for a fictitious binary file format, which uses a number of the built-in combinators:

```rust
const FILE_HEADER_SIZE: u8 = 6;
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
```

More examples of specific codecs can be found in the tests for `src/codec.rs` as well as in `tests/lib.rs`.

# License

`rcodec` is distributed under an MIT license.  See LICENSE for more details.
