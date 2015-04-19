//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
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
/// Example:
///
///   forcomp!({
///     foo <- Some(1u8);
///     bar <- None;
///   } yield { foo + bar });
///
/// expands to:
///
///   Some(1u8).and_then(|foo| {
///     None.map(|bar| {
///       foo + bar
///     })
///   })
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
// HList-related macros
//

#[macro_export]
macro_rules! hlist {
    {} => {
        $crate::hlist::HNil
    };
    { $head:expr } => {
        $crate::hlist::HCons($head, $crate::hlist::HNil)
    };
    { $head:expr, $($tail:expr),+ } => {
        $crate::hlist::HCons($head, hlist!($($tail),+))
    };
}

//
// ByteVector-related macros
//

/// Creates a new ByteVector from the given u8 values.
#[macro_export]
macro_rules! byte_vector {
    { $($byte:expr),* } => {
        $crate::byte_vector::from_vec(vec!($($byte),*))
    };
}

//
// Codec-related macros
//

/// Converts an HList of Codecs into a Codec that operates on an HList of values.
///
/// For example:
///   hcodec!(
///       { constant(c) } >>
///       { uint8       } ::
///       { uint8       }
///   )
///
/// translates to:
///   drop_left(constant(c), hlist_prepend_codec(uint8, hlist_prepend_codec(uint8, hnil_codec)))
///
/// which evaluates to:
///   Codec<HCons<u8, HCons<u8, HNil>>>
///
/// Note that we require braces around each element so that we have more freedom with operators.
/// Rust macro rules state that simple exprs (without the braces) can only be followed by
/// [ => , ; ] whereas blocks (with the braces) can be followed by any token like >> or ::.
#[macro_export]
macro_rules! hcodec {
    {} => {
        hnil_codec
    };
    { { $($head:tt)+ } } => {
        hlist_prepend_codec(hcodec_block!($($head)+), hnil_codec)
    };
    { { $($head:tt)+ } :: $($tail:tt)+ } => {
        hlist_prepend_codec(hcodec_block!($($head)+), hcodec!($($tail)+))
    };
    { { $($head:tt)+ } >> $($tail:tt)+ } => {
        drop_left(hcodec_block!($($head)+), hcodec!($($tail)+))
    };
    { { $($head:tt)+ } >>= |$v:ident| $fnbody:block } => {
        hlist_flat_prepend_codec(hcodec_block!($($head)+), |$v| $fnbody)
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

/// Suppose we have a struct like this:
///   struct Header {
///     foo: u8,
///     bar: u8
///   }
///
/// We want a codec that automatically converts HList values to Header fields, like this:
///   let header_codec = struct_codec!(Header from {uint8()} :: {uint8()});
#[macro_export]
macro_rules! struct_codec {
    { $stype:ident from $($hcodec:tt)+ } => {
        { struct_codec::<_, $stype, _>(hcodec!($($hcodec)+)) }
    };
}

/// Defines a struct that has derived impls for some common traits along with an `AsHList`
/// implementation that takes all fields into account.
#[macro_export]
macro_rules! record_struct {
    { $stype:ident, $($fieldname:ident: $fieldtype:ty),+ } => {
        #[derive(Debug, PartialEq, Eq, Clone)]
        #[AsHList]
        pub struct $stype {
            $($fieldname: $fieldtype),+
        }
    };
}
