use crate::fivegram::FIVEGRAM;
use crate::word::WordBytes;
use crate::{AsciiBitSet, Fivegram, Word};
use std::fmt::{Display, Formatter};

pub const PATTERN_COUNT: usize = 3usize.pow(FIVEGRAM as u32);
pub type Patterns = [Pattern; PATTERN_COUNT];

/**
Flat representation of the matching pattern.

Possible situations:
- Letter is at known position
- Letter is present, but not at given position
- Letter is absent in the word

Cases ignored:
- Repeated letter, one is at known position
 **/
#[derive(Debug, Default, Copy, Clone)]
#[repr(C)]
pub struct Pattern {
    pub match_word: Fivegram,
    pub present_letter: AsciiBitSet,
    pub absent_word: Fivegram,
    pub absent_letter: AsciiBitSet,
}

impl Display for Pattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,
               "Pattern {{ match_word: `{}`, present_letter: `{}`, absent_word: `{}`, absent_letter: `{}` }}", 
               self.match_word, self.present_letter, self.absent_word, self.absent_letter)
    }
}

impl Pattern {
    pub fn from_bytes(bytes: &WordBytes) -> Patterns {
        let mut patterns = [Pattern::default(); PATTERN_COUNT];

        for (j, p) in patterns.iter_mut().enumerate() {
            for i in 0..FIVEGRAM {
                // Pattern shifts one position
                match (j / (3usize.pow(i as u32))) % 3 {
                    // letter is at known position
                    0 => p.match_word.set_letter(bytes[i], i),

                    // letter present, but at wrong position
                    1 => {
                        p.absent_word.set_letter(bytes[i], i);
                        p.present_letter.set_letter(bytes[i]);
                    }

                    // letter is absent in the word
                    2 => {
                        p.absent_letter.set_letter(bytes[i]);
                    }

                    // universe is broken
                    _ => unreachable!(),
                }
            }
        }

        patterns
    }

    #[inline]
    pub fn matches_word(&self, word: &Word) -> bool {
        word.letters.is_superset(&self.present_letter)
            && word.letters.is_disjoint(&self.absent_letter)
            && word.fivegram.exact_match(&self.match_word)
            && !word.fivegram.any_pos_match(&self.absent_word)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::{AsciiBitSet, Pattern, Word};
    use proptest::prelude::*;

    #[test]
    fn proptest_is_matching_pattern() {
        proptest!(|(
            word in "[a-z]{5}",
            match_word in "[a-z_]{5}",
            absent_word in "[a-z_]{5}",
            match_letters in "[a-z]{0,5}",
            absent_letters in "[a-z]{0,16}"
            )| {
            let w = Word::from_str(&word);
            let mut pattern = Pattern::default();

            for (i, b) in match_word.as_bytes().iter().enumerate() {
                if *b != b'_' {
                    pattern.match_word.set_letter(*b, i);
                }
            }

            for (i, b) in absent_word.as_bytes().iter().enumerate() {
                if *b != b'_' {
                    pattern.absent_word.set_letter(*b, i);
                }
            }

            pattern.present_letter = AsciiBitSet::from_bytes(match_letters.as_bytes());
            pattern.absent_letter = AsciiBitSet::from_bytes(absent_letters.as_bytes());

            let word_letters: HashSet<char> = HashSet::from_iter(word.chars());
            let mhs: HashSet<char> = HashSet::from_iter(match_letters.chars());
            let ahs: HashSet<char> = HashSet::from_iter(absent_letters.chars());

            let positive_match_word = word.chars()
                .zip(match_word.chars())
                .all(|(l, r)| l == r || r == '_');

            let negative_match_word = word.chars()
                .zip(absent_word.chars())
                .all(|(l, r)| l != r || r == '_');

            let does_match = positive_match_word
                && negative_match_word
                && word_letters.is_superset(&mhs)
                && word_letters.is_disjoint(&ahs);

            assert_eq!(pattern.matches_word(&w), does_match);
        });
    }

    #[test]
    fn test_matching_empty_pattern() {
        let word = Word::from_str("hello");
        let pattern = Pattern::default();

        assert!(pattern.matches_word(&word));
    }

    #[test]
    fn test_failing_pattern() {
        let word = Word::from_str("ajaaa");
        let mut pattern = Pattern::default();
        pattern.absent_word.set_letter(b'j', 1);
        pattern.absent_word.set_letter(b'b', 2);

        assert!(!pattern.matches_word(&word));
    }

    #[test]
    fn test_matching_word_pattern() {
        let mut pattern = Pattern::default();
        pattern.match_word.set_letter(b'e', 1);
        pattern.match_word.set_letter(b'l', 3);

        let word = Word::from_str("hello");
        assert!(pattern.matches_word(&word));

        let non_matching_word = Word::from_str("aaaaa");
        assert!(!pattern.matches_word(&non_matching_word));
    }

    #[test]
    fn test_matching_negative_word_pattern() {
        let mut pattern = Pattern::default();
        pattern.absent_word.set_letter(b'a', 1);

        let word = Word::from_str("hello");
        assert!(pattern.matches_word(&word));

        let non_matching_word = Word::from_str("aaaaa");
        assert!(!pattern.matches_word(&non_matching_word));
    }

    #[test]
    fn test_matching_mix_word_pattern() {
        let mut pattern = Pattern::default();
        pattern.match_word.set_letter(b'l', 3);
        pattern.absent_word.set_letter(b'a', 1);

        let word = Word::from_str("hello");
        assert!(pattern.matches_word(&word));

        let non_matching_word = Word::from_str("aaaaa");
        assert!(!pattern.matches_word(&non_matching_word));
    }

    #[test]
    fn test_matching_letters_pattern() {
        let mut pattern = Pattern::default();
        pattern.present_letter.set_letter(b'l');

        let word = Word::from_str("hello");
        assert!(pattern.matches_word(&word));

        let non_matching_word = Word::from_str("aaaaa");
        assert!(!pattern.matches_word(&non_matching_word));
    }

    #[test]
    fn test_matching_negative_letters_pattern() {
        let mut pattern = Pattern::default();
        pattern.absent_letter.set_letter(b'a');

        let word = Word::from_str("hello");
        assert!(pattern.matches_word(&word));

        let non_matching_word = Word::from_str("aaaaa");
        assert!(!pattern.matches_word(&non_matching_word));
    }

    #[test]
    fn test_matching_mix_letters_pattern() {
        let mut pattern = Pattern::default();
        pattern.absent_letter.set_letter(b'a');
        pattern.present_letter.set_letter(b'e');

        let word = Word::from_str("hello");
        assert!(pattern.matches_word(&word));

        let non_matching_word = Word::from_str("aaaaa");
        assert!(!pattern.matches_word(&non_matching_word));
    }
}
