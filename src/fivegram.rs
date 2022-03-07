use crate::simd_pattern::{Mask, Simd};
use std::fmt::{Display, Formatter};

pub const FIVEGRAM: usize = 5;

/**
    Bit-packed 5-letter a-z ASCII word (26 < 2^5):

    empty = 0b00000
    a     = 0b00001
    ...
    z     = 0b11010
**/
#[derive(Debug, Default, Copy, Clone)]
#[repr(C)]
pub struct Fivegram {
    pub word: u32,
    pub letter_mask: u32,
}

impl Display for Fivegram {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s: String = (0..FIVEGRAM)
            .map(|i| {
                let masked = self.letter_mask >> (i * FIVEGRAM) & 0b11111 == 0;
                if masked {
                    '_'
                } else {
                    let b = (self.word >> (i * FIVEGRAM) & 0b11111) as u8;
                    char::from(b - 1 + b'a')
                }
            })
            .collect();

        write!(f, "{}", s)
    }
}

impl Fivegram {
    #[inline]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        assert!(bytes.len() <= FIVEGRAM);

        let mut res = Self::default();
        for (i, b) in bytes.into_iter().enumerate() {
            res.set_letter(*b, i);
        }

        res
    }

    #[inline]
    pub fn set_letter(&mut self, l: u8, pos: usize) {
        self.word |= ((l - b'a' + 1) as u32) << (pos * FIVEGRAM);
        self.letter_mask |= 0b11111 << (pos * FIVEGRAM);
    }

    #[inline]
    pub fn exact_match(&self, pattern: &Self) -> bool {
        self.word & pattern.letter_mask ^ pattern.word == 0
    }

    #[inline]
    pub fn exact_match_simd(word: &Simd, letter_mask: &Simd, pattern: &Simd) -> Simd {
        word & letter_mask ^ pattern
    }

    #[inline]
    pub fn any_pos_match(&self, pattern: &Self) -> bool {
        let intersection =
            ((self.word & pattern.letter_mask) ^ pattern.word) | !pattern.letter_mask;

        intersection & 0b11111 == 0
            || intersection >> 5 & 0b11111 == 0
            || intersection >> 10 & 0b11111 == 0
            || intersection >> 15 & 0b11111 == 0
            || intersection >> 20 & 0b11111 == 0
    }

    #[inline]
    pub fn any_pos_match_simd(word: &Simd, letter_mask: &Simd, pattern: &Simd) -> Mask {
        let intersection = (word & letter_mask ^ pattern) | !letter_mask.clone();

        let mut acc = Mask::splat(false);
        let zeros = Simd::splat(0);
        let first_five_mask = Simd::splat(0b11111);

        for shift in [
            Simd::splat(0),
            Simd::splat(5),
            Simd::splat(10),
            Simd::splat(15),
            Simd::splat(20),
        ] {
            acc |= ((intersection >> shift) & first_five_mask).lanes_eq(zeros);
        }

        acc
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;

    use crate::simd_pattern::Simd;
    use crate::simd_pattern::SIMD_WIDTH;

    use super::Fivegram;

    fn fivegram_from_pattern(pattern: &str) -> Fivegram {
        let mut fg = Fivegram::default();
        pattern.as_bytes().iter().enumerate().for_each(|(i, b)| {
            if *b != b'_' {
                fg.set_letter(*b, i);
            }
        });

        fg
    }

    #[test]
    fn proptest_exact_match_simd() {
        proptest!(|(word in "[a-z]{5}", pattern in ["[a-z_]{5}"; SIMD_WIDTH])| {
            let expected: [bool; SIMD_WIDTH] = pattern.iter().map(|p| {
                word.chars().zip(p.chars()).all(|(l, r)| l == r || r == '_')
            }).collect::<Vec<bool>>().try_into().unwrap();

            let word = Simd::splat(Fivegram::from_bytes(word.as_bytes()).word);

            let pattern = pattern.map(|p| fivegram_from_pattern(&p));
            let pattern_words = Simd::from_array(pattern.map(|fg| fg.word));
            let pattern_masks = Simd::from_array(pattern.map(|fg| fg.letter_mask));

            let res = Fivegram::exact_match_simd(&word, &pattern_masks, &pattern_words);
            let zeros = Simd::splat(0);
            assert_eq!(res.lanes_eq(zeros).to_array(), expected)
        })
    }

    #[test]
    fn proptest_exact_match() {
        proptest!(|(word in "[a-z]{5}", pattern in "[a-z_]{5}")| {
            let w = Fivegram::from_bytes(word.as_bytes());
            let p = fivegram_from_pattern(&pattern);

            let does_match = word.chars()
                .zip(pattern.chars())
                .all(|(l, r)| l == r || r == '_');

            assert_eq!(w.exact_match(&p), does_match);
        });
    }

    #[test]
    fn proptest_any_pos_match_simd() {
        proptest!(|(word in "[a-z]{5}", pattern in ["[a-z_]{5}"; SIMD_WIDTH])| {
            let expected: [bool; SIMD_WIDTH] = pattern.iter().map(|p| {
                word.chars().zip(p.chars()).any(|(l, r)| l == r && r != '_')
            }).collect::<Vec<bool>>().try_into().unwrap();

            let word = Simd::splat(Fivegram::from_bytes(word.as_bytes()).word);

            let pattern = pattern.map(|p| fivegram_from_pattern(&p));
            let pattern_words = Simd::from_array(pattern.map(|fg| fg.word));
            let pattern_masks = Simd::from_array(pattern.map(|fg| fg.letter_mask));

            let res = Fivegram::any_pos_match_simd(&word, &pattern_masks, &pattern_words);
            assert_eq!(res.to_array(), expected)
        })
    }

    #[test]
    fn proptest_any_pos_match() {
        proptest!(|(word in "[a-z]{5}", pattern in "[a-z_]{5}")| {
            let w = Fivegram::from_bytes(word.as_bytes());
            let p = fivegram_from_pattern(&pattern);

            let does_match = word.chars()
                .zip(pattern.chars())
                .any(|(l, r)| l == r && r != '_');

            assert_eq!(w.any_pos_match(&p), does_match);
        });
    }

    #[test]
    fn test_from_bytes() {
        let fg = Fivegram::from_bytes("abcde".as_bytes());

        assert_eq!(fg.word, 0b00_00000_00101_00100_00011_00010_00001);
    }

    #[test]
    fn test_matches_full() {
        let l = Fivegram::from_bytes("abcde".as_bytes());
        let r = Fivegram::from_bytes("abcde".as_bytes());

        assert!(l.exact_match(&r));
    }

    #[test]
    fn test_matches_prefix() {
        let l = Fivegram::from_bytes("abcde".as_bytes());
        let r = Fivegram::from_bytes("abc".as_bytes());

        assert!(l.exact_match(&r));
    }

    #[test]
    fn test_doesnt_match_wrong_letters() {
        let l = Fivegram::from_bytes("cbcde".as_bytes());
        let r = Fivegram::from_bytes("a".as_bytes());

        assert!(!l.exact_match(&r));
    }

    #[test]
    fn test_matches_with_holes() {
        let l = Fivegram::from_bytes("abcde".as_bytes());
        let mut r = Fivegram::default();
        r.set_letter(b'b', 1);
        r.set_letter(b'd', 3);

        assert!(l.exact_match(&r));
    }
}
