use crate::simd_pattern::Simd;
use std::fmt::{Display, Formatter};

/**
    Lower-case ASCII bit-set, to quickly check if letter i
**/
#[derive(Debug, Default, Copy, Clone)]
#[repr(transparent)]
pub struct AsciiBitSet {
    pub set: u32,
}

impl Display for AsciiBitSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s: String = (0..26)
            .flat_map(|i| {
                let is_set = self.set >> i & 0b1 == 1;
                if is_set {
                    Some(char::from(i + b'a'))
                } else {
                    None
                }
            })
            .collect();
        write!(f, "{}", s)
    }
}

impl AsciiBitSet {
    #[inline]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut set = Self::default();
        for b in bytes {
            set.set |= 1 << (b - b'a');
        }

        set
    }

    #[inline]
    pub fn set_letter(&mut self, l: u8) {
        self.set |= 1 << (l - b'a')
    }

    #[inline]
    pub fn is_superset(&self, set: &Self) -> bool {
        self.set & set.set ^ set.set == 0
    }

    #[inline]
    pub fn is_superset_simd(a: &Simd, b: &Simd) -> Simd {
        a & b ^ b
    }

    #[inline]
    pub fn is_disjoint(&self, set: &Self) -> bool {
        self.set & set.set == 0
    }

    #[inline]
    pub fn is_disjoint_simd(a: &Simd, b: &Simd) -> Simd {
        a & b
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use std::collections::HashSet;

    use proptest::prelude::*;

    use crate::simd_pattern::Simd;
    use crate::simd_pattern::SIMD_WIDTH;

    use super::AsciiBitSet;

    #[test]
    fn proptest_is_superset_simd() {
        proptest!(|(left in ["[a-z]{0,16}"; SIMD_WIDTH], right in ["[a-z]{0,16}"; SIMD_WIDTH])| {
            let expected: [bool; SIMD_WIDTH] = left.iter().zip(right.iter()).map(|(left, right)| {
                let lhs: HashSet<char> = HashSet::from_iter(left.chars());
                let rhs: HashSet<char> = HashSet::from_iter(right.chars());

                lhs.is_superset(&rhs)
            }).collect::<Vec<bool>>().try_into().unwrap();

            let left = Simd::from_array(left.map(|l| AsciiBitSet::from_bytes(l.as_bytes()).set));
            let right = Simd::from_array(right.map(|r| AsciiBitSet::from_bytes(r.as_bytes()).set));

            let res = AsciiBitSet::is_superset_simd(&left, &right);
            let zero = Simd::splat(0);
            assert_eq!(res.lanes_eq(zero).to_array(), expected);
        });
    }

    #[test]
    fn proptest_is_superset() {
        proptest!(|(left in "[a-z]{0,16}", right in "[a-z]{0,16}")| {
            let l = AsciiBitSet::from_bytes(left.as_bytes());
            let r = AsciiBitSet::from_bytes(right.as_bytes());

            let lhs: HashSet<char> = HashSet::from_iter(left.chars());
            let rhs: HashSet<char> = HashSet::from_iter(right.chars());

            assert_eq!(l.is_superset(&r), lhs.is_superset(&rhs));
        });
    }

    #[test]
    fn proptest_is_disjoint_simd() {
        proptest!(|(left in ["[a-z]{0,16}"; SIMD_WIDTH], right in ["[a-z]{0,16}"; SIMD_WIDTH])| {
            let expected: [bool; SIMD_WIDTH] = left.iter().zip(right.iter()).map(|(left, right)| {
                let lhs: HashSet<char> = HashSet::from_iter(left.chars());
                let rhs: HashSet<char> = HashSet::from_iter(right.chars());

                lhs.is_disjoint(&rhs)
            }).collect::<Vec<bool>>().try_into().unwrap();

            let left = Simd::from_array(left.map(|l| AsciiBitSet::from_bytes(l.as_bytes()).set));
            let right = Simd::from_array(right.map(|r| AsciiBitSet::from_bytes(r.as_bytes()).set));

            let res = AsciiBitSet::is_disjoint_simd(&left, &right);
            let zero = Simd::splat(0);
            assert_eq!(res.lanes_eq(zero).to_array(), expected);
        });
    }

    #[test]
    fn proptest_is_disjoint() {
        proptest!(|(left in "[a-z]{0,16}", right in "[a-z]{0,16}")| {
            let l = AsciiBitSet::from_bytes(left.as_bytes());
            let r = AsciiBitSet::from_bytes(right.as_bytes());

            let lhs: HashSet<char> = HashSet::from_iter(left.chars());
            let rhs: HashSet<char> = HashSet::from_iter(right.chars());

            assert_eq!(l.is_disjoint(&r), lhs.is_disjoint(&rhs));
        });
    }

    #[test]
    fn from_bytes() {
        let s = AsciiBitSet::from_bytes("abcde".as_bytes());
        assert_eq!(s.set, 0b11111);

        let s = AsciiBitSet::from_bytes("zyxwv".as_bytes());
        assert_eq!(s.set, 0b0011_1110_0000_0000_0000_0000_0000);

        let s = AsciiBitSet::from_bytes("abcdefghijklmnopqrstuvwxyz".as_bytes());
        assert_eq!(s.set, 0b0011_1111_1111_1111_1111_1111_1111);
    }

    #[test]
    fn is_superset() {
        let l = AsciiBitSet::from_bytes("abcde".as_bytes());
        let r = AsciiBitSet::from_bytes("bd".as_bytes());

        assert!(l.is_superset(&r));
        assert!(!r.is_superset(&l));
    }

    #[test]
    fn not_is_superset() {
        let l = AsciiBitSet::from_bytes("abcde".as_bytes());
        let r = AsciiBitSet::from_bytes("abcz".as_bytes());

        assert!(!l.is_superset(&r));
        assert!(!r.is_superset(&l));
    }
}
