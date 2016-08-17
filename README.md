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

Here's an example of a more complex codec for a fictitious binary packet format, which uses a number of the built-in combinators:

```rust
#[derive(Debug, PartialEq, Eq, Clone)]
#[HListSupport]
struct PacketHeader {
    version: u8,
    port: u16,
    checksum: u16,
    data_len: u16
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[HListSupport]
struct Packet {
    header: PacketHeader,
    flags: u64,
    data: Vec<u8>
}

let magic = byte_vector!(0xCA, 0xFE, 0xCA, 0xFE);

let header_codec = struct_codec!(
    PacketHeader from
    { "version"      => uint8  } ::
    { "port"         => uint16 } ::
    { "checksum"     => uint16 } ::
    { "data_len"     => uint16 }
);

let packet_codec = struct_codec!(
    Packet from
    { "magic"           => constant(&magic) } >>
    { "padding"         => ignore(4)        } >>
    { "header"          => header_codec     } >>= |hdr| {
        hcodec!(
            { "flags"    => uint64                                       } ::
            { "data"     => eager(bytes((hdr.data_len - 8u16) as usize)) })
    }
);
```

More examples of specific codecs can be found in the tests for `src/codec.rs` as well as in `tests/lib.rs`.

# License

`rcodec` is distributed under an MIT license.  See LICENSE for more details.
