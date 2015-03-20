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
