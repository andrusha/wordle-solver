use crate::pattern::{Patterns, PATTERN_COUNT};
use crate::{AsciiBitSet, Fivegram, Word};

// since ARM-neon only has 128-bit SIMD registers
pub const SIMD_WIDTH: usize = 4;
pub const SIMD_PATTERN_COUNT: usize = PATTERN_COUNT.div_ceil(SIMD_WIDTH);

pub type Simd = core_simd::Simd<u32, SIMD_WIDTH>;
pub type FreqSimd = core_simd::Simd<i32, SIMD_WIDTH>;
pub type Mask = core_simd::Mask<i32, SIMD_WIDTH>;
pub type SimdPatterns = [SimdPattern; SIMD_PATTERN_COUNT];

#[derive(Debug, Default, Copy, Clone)]
// #[repr(C)]
pub struct SimdPattern {
    absent_letter: Simd,
    present_letter: Simd,
    match_word: Simd,
    match_word_mask: Simd,
    absent_word: Simd,
    absent_word_mask: Simd,
}

impl SimdPattern {
    pub fn from_patterns(patterns: &Patterns) -> SimdPatterns {
        patterns
            .chunks(SIMD_WIDTH)
            .map(|chunk| {
                let mut absent_letters = [0u32; SIMD_WIDTH];
                let mut present_letters = [0u32; SIMD_WIDTH];
                let mut match_word = [0u32; SIMD_WIDTH];
                let mut match_word_mask = [0u32; SIMD_WIDTH];
                let mut absent_word = [0u32; SIMD_WIDTH];
                let mut absent_word_mask = [0u32; SIMD_WIDTH];

                for (i, pm) in chunk.into_iter().enumerate() {
                    absent_letters[i] = pm.absent_letter.set;
                    present_letters[i] = pm.present_letter.set;
                    match_word[i] = pm.match_word.word;
                    match_word_mask[i] = pm.match_word.letter_mask;
                    absent_word[i] = pm.absent_word.word;
                    absent_word_mask[i] = pm.absent_word.letter_mask;
                }

                SimdPattern {
                    absent_letter: Simd::from_array(absent_letters),
                    present_letter: Simd::from_array(present_letters),
                    match_word: Simd::from_array(match_word),
                    match_word_mask: Simd::from_array(match_word_mask),
                    absent_word: Simd::from_array(absent_word),
                    absent_word_mask: Simd::from_array(absent_word_mask),
                }
            })
            .collect::<Vec<SimdPattern>>()
            .try_into()
            .unwrap()
    }

    #[inline]
    pub fn matches_word(&self, word: &Word) -> Mask {
        let letters = Simd::splat(word.letters.set);
        let is_superset = AsciiBitSet::is_superset_simd(&letters, &self.present_letter);
        let is_disjoint = AsciiBitSet::is_disjoint_simd(&letters, &self.absent_letter);

        let word = Simd::splat(word.fivegram.word);
        let is_exact_match =
            Fivegram::exact_match_simd(&word, &self.match_word_mask, &self.match_word);
        let is_any_letter_match =
            Fivegram::any_pos_match_simd(&word, &self.absent_word_mask, &self.absent_word);

        let zeros = Simd::splat(0);
        is_superset.lanes_eq(zeros)
            & is_disjoint.lanes_eq(zeros)
            & is_exact_match.lanes_eq(zeros)
            & !is_any_letter_match
    }
}

#[cfg(test)]
mod tests {
    use crate::simd_pattern::SIMD_WIDTH;
    use crate::word::wordbytes_from_str;
    use crate::{Pattern, SimdPattern, Word};
    use core_simd::Simd;
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;

    #[test]
    fn simd_logical_ops() {
        let zero = Simd::splat(0);

        let x: Simd<u32, 4> = Simd::from([1, 0, 2, 3]);
        let logic_x = x.lanes_eq(zero);

        assert_eq!(logic_x.to_array(), [false, true, false, false]);
        assert_eq!(logic_x.to_int().to_array(), [0, -1, 0, 0]);

        let y: Simd<u32, 4> = Simd::from([0, 10, 0, 20]);
        let logic_y = y.lanes_eq(zero);

        assert_eq!(logic_y.to_array(), [true, false, true, false]);
        assert_eq!(logic_y.to_int().to_array(), [-1, 0, -1, 0]);

        let logic_xy = logic_x | logic_y;
        assert_eq!(logic_xy.to_array(), [true, true, true, false]);
        assert_eq!(logic_xy.to_int().to_array(), [-1, -1, -1, 0]);
    }

    // Considering CPU-naive implementation to be the reference
    #[test]
    fn proptest_single_pattern_matches_word() {
        proptest!(|(pattern_word in "[a-z]{5}", match_word in "[a-z]{5}")| {
            let patterns = Pattern::from_bytes(&wordbytes_from_str(&pattern_word));
            let simd_patterns = SimdPattern::from_patterns(&patterns);
            let word = Word::from_str(&match_word);

            let cpu_match: Vec<bool> = patterns[0..SIMD_WIDTH].iter().map(|p| p.matches_word(&word)).collect();
            let simd_match = simd_patterns.first().unwrap().matches_word(&word).to_array();

            assert_eq!(cpu_match, simd_match, "CPU Patterns: [{}, {}, {}, {}], SIMD Pattern: {:?}", patterns[0], patterns[1], patterns[2], patterns[3], simd_patterns.first().unwrap());
        });
    }
}
