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
// Codec-related macros
//

/// Converts an HList of Codecs into a Codec that operates on an HList of values.
///
/// For example:
///       hcodec!(uint8(), uint8())
///
/// translates to:
///       hlist_prepend_codec(uint8(), hlist_prepend_codec(uint8(), hnil_codec()))
///
/// which evaluates to:
///       Codec<HCons<u8, HCons<u8, HNil>>>
#[macro_export]
macro_rules! hcodec {
    {} => {
        hnil_codec()
    };
    { $head:expr } => {
        hlist_prepend_codec($head, hnil_codec())
    };
    { $head:expr, $($tail:expr),+ } => {
        hlist_prepend_codec($head, hcodec!($($tail),+))
    };
    { $($head:expr),+ : $tail:expr } => {
        hcodec_expr_tail!({ $tail } $($head),+)
    };
}

macro_rules! hcodec_expr_tail {
    { { $tail:expr } } => {
        $tail
    };
    { { $tail:expr } $head:expr } => {
        hlist_prepend_codec($head, $tail)
    };
    { { $tail:tt } $head:expr, $($rest:tt),+ } => {
        hlist_prepend_codec($head, hcodec_expr_tail!({ $tail } $($rest),+))
    };
}
