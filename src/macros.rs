//
// Copyright (c) 2015-2019 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// HList macro implementations based on:
//   https://github.com/epsilonz/shoggoth.rs
//

//
// Scala-style for/yield macros
//

/// Scala-like for-comprehension macro.
///
/// # Examples
///
/// ```
/// use rcodec::codec::*;
///
/// # fn main() {
/// let x = forcomp!({
///     foo <- Some(1u8);
///     bar <- None;
/// } yield { foo + bar });
/// assert!(x.is_none());
/// # }
/// ```
///
/// This is equivalent to:
///
/// ```
/// let x = Some(1u8).and_then(|foo| {
///     None.map(|bar| {
///         foo + bar
///     })
/// });
/// assert!(x.is_none());
/// ```
macro_rules! forcomp {
    { { $($v:ident <- $e:expr;)+ } yield $yld:block } => {
        forcomp_stmts!($yld, $($v, $e),+)
    };
}

macro_rules! forcomp_stmts {
    { $yld:block, $v:ident, $e:expr } => {
        $e.map(move |$v| $yld)
    };
    { $yld:block, $v:ident, $e:expr, $($tv:ident, $te:expr),+ } => {
        $e.and_then(move |$v| {
            forcomp_stmts!($yld, $($tv, $te),+)
        })
    };
}

//
// ByteVector-related macros
//

/// Creates a new `ByteVector` from the given `u8` values.
///
/// # Examples
///
/// ```
/// use rcodec::byte_vector;
///
/// # fn main() {
/// let bv = byte_vector!(1, 2, 3, 4);
/// assert_eq!(bv, byte_vector::from_vec(vec!(1, 2, 3, 4)));
/// # }
/// ```
#[macro_export]
macro_rules! byte_vector {
    { $($byte:expr),* } => {
        $crate::byte_vector::from_vec(vec!($($byte),*))
    };
}

//
// Codec-related macros
//

/// Converts an `HList` of `Codec`s into a `Codec` that operates on an `HList` of values.
///
/// Note that we require braces around each element so that we have more freedom with operators.
/// Rust macro rules state that simple exprs (without the braces) can only be followed by
/// `=> , ;` whereas blocks (with the braces) can be followed by any token like `>>` or `::`.
///
/// # Examples
///
/// ```
/// use hlist::*;
/// use rcodec::{byte_vector, hcodec};
/// use rcodec::codec::*;
///
/// # fn main() {
/// let c = byte_vector!(0xCA, 0xFE);
/// let codec = hcodec!(
///     { "magic"  => constant(&c) } >>
///     { "field1" => uint8        } ::
///     { "field2" => uint8        }
/// );
///
/// let bytes = byte_vector!(0xCA, 0xFE, 0x01, 0x02);
/// let decoded = codec.decode(&bytes).unwrap().value;
/// assert_eq!(decoded, hlist!(1, 2));
/// # }
/// ```
#[macro_export]
macro_rules! hcodec {
    {} => {
        hnil_codec
    };
    { { $($head:tt)+ } } => {
        hlist_prepend_codec($crate::hcodec_block!($($head)+), hnil_codec())
    };
    { { $($head:tt)+ } :: $($tail:tt)+ } => {
        hlist_prepend_codec($crate::hcodec_block!($($head)+), $crate::hcodec!($($tail)+))
    };
    { { $($head:tt)+ } >> $($tail:tt)+ } => {
        drop_left($crate::hcodec_block!($($head)+), $crate::hcodec!($($tail)+))
    };
    { { $($head:tt)+ } >>= |$v:ident| $fnbody:block } => {
        hlist_flat_prepend_codec($crate::hcodec_block!($($head)+), |$v| $fnbody)
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! hcodec_block {
    { $ctx:expr => $codec:expr } => {
        with_context($ctx, $codec)
    };
    { $codec:expr } => {
        $codec
    };
}

/// Shorthand for creating a `Codec` for a struct.
///
/// The given struct must support `HList` conversions, either by using the `HListSupport` attribute
/// or by manually implementing the `FromHList` and `ToHList` traits.
///
/// # Examples
///
/// ```
/// use hlist::*;
/// use rcodec::{byte_vector, struct_codec};
/// use rcodec::codec::*;
///
/// #[derive(Debug, PartialEq, Eq, HListSupport)]
/// pub struct Header {
///     foo: u8,
///     bar: u32
/// }
///
/// # fn main() {
/// let magic = byte_vector!(0xCA, 0xFE);
/// let header_codec = struct_codec!(
///     Header from
///     { "magic" => constant(&magic) } >>
///     { "foo"   => uint8            } ::
///     { "junk"  => ignore(2)        } >>
///     { "bar"   => uint32           }
/// );
///
/// let bytes = byte_vector!(0xCA, 0xFE, 0x07, 0xBE, 0xEF, 0x00, 0x00, 0x00, 0x06);
/// let header = header_codec.decode(&bytes).unwrap().value;
/// assert_eq!(header, Header { foo: 7, bar: 6 });
/// # }
/// ```
#[macro_export]
macro_rules! struct_codec {
    { $stype:ident from $($hcodec:tt)+ } => {
        { struct_codec::<_, $stype, _>($crate::hcodec!($($hcodec)+)) }
    };
}

/// Defines a struct that has derived impls for some common traits along with implementations
/// of the `FromHList` and `ToHList` traits, taking all fields into account.
///
/// # Examples
///
/// ```
/// use hlist::*;
/// use rcodec::*;
///
/// record_struct!(
///     TestStruct,
///     foo: u8,
///     bar: u32
/// );
///
/// # fn main() {
/// let hlist = hlist!(7u8, 666u32);
/// let s = TestStruct::from_hlist(hlist);
/// assert_eq!(s, TestStruct { foo: 7, bar: 666 });
/// # }
/// ```
#[macro_export]
macro_rules! record_struct {
    { $stype:ident, $($fieldname:ident: $fieldtype:ty),+ } => {
        #[derive(Debug, PartialEq, Eq, Clone, HListSupport)]
        pub struct $stype {
            $($fieldname: $fieldtype),+
        }
    };
}
