//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This implementation based on List type from:
//   https://github.com/epsilonz/shoggoth.rs
//

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HNil;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HCons<H, T>(pub H, pub T);

pub trait HList {
    fn cons<X>(self, x: X) -> HCons<X, Self> where Self: Sized {
        HCons(x, self)
    }
}

impl HList for HNil {
}

impl<H, T> HList for HCons<H, T> {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hlist_macros_should_work() {
        {
            let hlist1 = HNil;
            let hlist2 = hlist!();
            assert_eq!(hlist1, hlist2);
        }

        {
            let hlist1 = HCons(1u8, HNil);
            let hlist2 = hlist!(1u8);
            assert_eq!(hlist1, hlist2);
        }

        {
            let hlist1 = HCons(1u8, HCons(2i32, HCons("three", HNil)));
            let hlist2 = hlist!(1u8, 2i32, "three");
            assert_eq!(hlist1, hlist2);
        }
    }
}
