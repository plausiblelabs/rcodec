//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// HList macro implementations based on:
//   https://github.com/epsilonz/shoggoth.rs
//

//
// HList-related macros
//

#[macro_export]
macro_rules! hlist {
    {} => {
        $crate::hlist::HNil
    };
    // TODO: What does this rule do?
    {=> $($elem:tt)+ } => {
        hlist_pat!($($elem)+)
    };
    { $head:expr } => {
        $crate::hlist::HCons($head, $crate::hlist::HNil)
    };
    { $head:expr, $($tail:expr),+ } => {
        $crate::hlist::HCons($head, hlist!($($tail),+))
    };
    { $($head:expr),+ : $tail:expr } => {
        hlist_expr_tail!({ $tail } $($head),+)
    };
}

macro_rules! hlist_expr_tail {
    { { $tail:expr } } => {
        $tail
    };
    { { $tail:expr } $head:expr } => {
        $crate::hlist::HCons($head, $tail)
    };
    { { $tail:tt } $head:expr, $($rest:tt),+ } => {
        $crate::hlist::HCons($head, hlist_expr_tail!({ $tail } $($rest),+))
    };
}

macro_rules! hlist_pat {
    {} => {
        $crate::hlist::HNil
    };
    { $head:pat } => {
        $crate::hlist::HCons($head, $crate::hlist::HNil)
    };
    { $head:pat, $($rest:tt),+ } => {
        $crate::hlist::HCons($head, hlist_pat!($($rest),+))
    };
    { $($head:pat),+ : $tail:pat } => {
        hlist_pat_tail!({ $tail } $($head),+)
    };
}

macro_rules! hlist_pat_tail {
    { { $tail:pat } } => {
        $tail
    };
    { { $tail:pat } $head:pat } => {
        $crate::hlist::HCons($head, $tail)
    };
    { { $tail:tt } $head:pat, $($rest:tt),+ } => {
        $crate::hlist::HCons($head, hlist_pat_tail!({ $tail } $($rest),+))
    };
}

//
// ByteVector-related macros
//

/// Creates a new ByteVector from the given u8 values.
#[macro_export]
macro_rules! byte_vector {
    { $($byte:expr),* } => {
        $crate::byte_vector::buffered(&vec!($($byte),*))
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
    { $head:block } => {
        hlist_prepend_codec($head, hnil_codec)
    };
    { $head:block :: $($tail:tt)+ } => {
        hlist_prepend_codec($head, hcodec!($($tail)+))
    };
    { $head:block >> $($tail:tt)+ } => {
        drop_left($head, hcodec!($($tail)+))
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

/// Shorthand for defining record structs that support HList conversions.
#[macro_export]
macro_rules! record_struct_with_hlist_type {
    // Note that sadly, macros cannot expand to a type, so we have to manually pass in the HList type here
    // instead of just generating it from the list of field types.  We provide the record_struct! compiler
    // plugin as a more convenient frontend to this macro, since it can take care of building the HList type.
    { $stype:ident, $hlisttype:ty, $($fieldname:ident: $fieldtype:ty),+ } => {
        #[derive(Debug, PartialEq, Eq)]
        pub struct $stype {
            $($fieldname: $fieldtype),+
        }

        #[allow(dead_code)]
        impl AsHList<$hlisttype> for $stype {
            fn from_hlist(hlist: &$hlisttype) -> Self {
                match *hlist {
                    record_struct_hlist_pattern!($($fieldname),+) => $stype { $($fieldname: $fieldname),+ }
                }
            }
            
            fn to_hlist(&self) -> $hlisttype {
                hlist!($(self.$fieldname),+)
            }
        }
    };
}

macro_rules! record_struct_hlist_pattern {
    { $head:ident } => {
        $crate::hlist::HCons($head, $crate::hlist::HNil)
    };
    { $head:ident, $($tail:ident),+ } => {
        $crate::hlist::HCons($head, record_struct_hlist_pattern!($($tail),+))
    };
}
