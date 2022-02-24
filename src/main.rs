#![feature(portable_simd)]
#![feature(int_roundings)]

use clap::{ArgEnum, Parser};
use rayon::prelude::*;

use ascii_bit_set::AsciiBitSet;
use fivegram::Fivegram;
use pattern::{Pattern, Patterns};
use word::Word;

use crate::pattern::PATTERN_COUNT;
use crate::simd_pattern::{FreqSimd, SimdPattern, SimdPatterns, SIMD_WIDTH};

mod ascii_bit_set;
mod fivegram;
mod pattern;
mod simd_pattern;
mod word;

const WORD_COUNT: usize = 12972;

#[derive(ArgEnum, Clone)]
enum Implementation {
    CPU,
    SIMD,
}

#[derive(Parser)]
#[clap(version = "0.1", author = "Andrew Korzhuev <korzhuev@andrusha.me>")]
struct Cli {
    #[clap(arg_enum)]
    implementation: Implementation,
}

fn main() {
    let cli = Cli::parse();

    let now = std::time::Instant::now();

    let all_words = all_words();
    let all_patterns = all_patterns(&all_words);

    let infs: Vec<f32>;
    match cli.implementation {
        Implementation::CPU => {
            infs = match_freq(&all_words, &all_patterns);
        }
        Implementation::SIMD => {
            let all_simd_patterns = all_simd_patterns(&all_patterns);
            infs = match_freq_simd(&all_words, &all_simd_patterns);
        }
    }

    let idx = top_k_indices::<10>(&infs);

    println!("Top choices by information gain:");
    for i in idx {
        println!("{}: {}", all_words[i].to_str(), infs[i]);
    }

    let time = now.elapsed().as_millis();
    println!("Time: {}ms", time);
}

fn all_words() -> Vec<Word> {
    include_str!("../dict.txt")
        .lines()
        .map(Word::from_str)
        .collect()
}

fn all_patterns(words: &[Word]) -> Vec<Patterns> {
    let words = &words[0..WORD_COUNT];

    words
        .iter()
        .map(|word| Pattern::from_bytes(&word.bytes))
        .collect()
}

fn all_simd_patterns(patterns: &[Patterns]) -> Vec<SimdPatterns> {
    let patterns = &patterns[0..WORD_COUNT];

    patterns.iter().map(SimdPattern::from_patterns).collect()
}

fn top_k_indices<const K: usize>(keys: &[f32]) -> [usize; K] {
    let mut idx = [0; K];
    for i in 1..keys.len() {
        if keys[idx[0]] <= keys[i] {
            idx.copy_within(0..K - 1, 1);
            idx[0] = i;
        }
    }

    idx
}

fn counts_to_entropy(counts: &[usize]) -> f32 {
    counts
        .into_iter()
        .filter(|&&f| f != 0)
        .map(|&f| f as f32 / WORD_COUNT as f32)
        .map(|p| -p * p.log2())
        .sum()
}

fn match_freq(words: &[Word], patterns: &[Patterns]) -> Vec<f32> {
    patterns
        .into_par_iter()
        .map(|patterns| match_patterns(&words, patterns))
        .map(|bins| counts_to_entropy(&bins))
        .collect()
}

fn match_patterns(words: &[Word], patterns: &Patterns) -> [usize; PATTERN_COUNT] {
    let words = &words[0..WORD_COUNT];
    let mut matches = [0; PATTERN_COUNT];

    for (i, pattern) in patterns.into_iter().enumerate() {
        for other in words {
            if pattern.matches_word(&other) {
                matches[i] += 1
            }
        }
    }

    matches
}

fn match_freq_simd(words: &[Word], patterns: &[SimdPatterns]) -> Vec<f32> {
    patterns
        .into_par_iter()
        .map(|patterns| match_patterns_simd(&words, &patterns))
        .map(|bins| counts_to_entropy(&bins))
        .collect()
}

fn match_patterns_simd(words: &[Word], patterns: &SimdPatterns) -> [usize; PATTERN_COUNT] {
    let words = &words[0..WORD_COUNT];
    let mut matches = [0; PATTERN_COUNT];

    for (i, pattern) in patterns.into_iter().enumerate() {
        let mut f = FreqSimd::splat(0);
        for other in words {
            let mask = pattern.matches_word(&other);
            f -= mask.to_int();
        }
        let offset = i * SIMD_WIDTH;
        let end_offset = (offset + SIMD_WIDTH).min(PATTERN_COUNT);
        let slice_length = (PATTERN_COUNT - offset).min(SIMD_WIDTH);
        matches[offset..end_offset]
            .copy_from_slice(&f.to_array().map(|x| x.unsigned_abs() as usize)[..slice_length]);
    }

    matches
}

#[cfg(test)]
mod tests {
    use crate::{all_patterns, all_simd_patterns, all_words, match_patterns, match_patterns_simd};
    use std::collections::HashSet;

    const KNOWN_WORD: &str = "sorel";
    const KNOWN_BINS: [usize; 182] = [
        986, 923, 366, 324, 193, 129, 375, 349, 54, 374, 256, 70, 200, 70, 22, 74, 54, 8, 189, 170,
        57, 39, 17, 14, 77, 65, 13, 636, 403, 203, 131, 44, 28, 97, 41, 11, 388, 148, 41, 83, 18,
        9, 16, 6, 1, 122, 66, 19, 12, 9, 3, 18, 7, 368, 369, 107, 19, 11, 5, 143, 96, 6, 311, 76,
        36, 20, 3, 1, 69, 16, 3, 73, 43, 22, 4, 2, 11, 11, 5, 434, 326, 106, 123, 66, 31, 114, 119,
        17, 54, 11, 4, 15, 2, 1, 13, 7, 1, 23, 30, 1, 2, 5, 2, 321, 158, 58, 63, 10, 4, 43, 7, 6,
        39, 9, 5, 3, 11, 4, 1, 125, 94, 15, 6, 4, 1, 26, 24, 3, 57, 6, 4, 5, 1, 5, 2, 1, 5, 4, 1,
        94, 6, 33, 21, 2, 10, 31, 2, 45, 3, 16, 1, 3, 11, 1, 3, 9, 1, 35, 1, 15, 3, 1, 2, 1, 8, 2,
        5, 1, 2, 53, 4, 9, 1, 1, 15, 2, 13, 1, 1, 3, 4, 1,
    ];

    #[test]
    fn known_bins_cpu() {
        let words = all_words();
        let patterns = all_patterns(&words);

        let sorel_idx = words
            .iter()
            .enumerate()
            .find(|(_, w)| w.to_str() == KNOWN_WORD)
            .unwrap()
            .0;
        let sorel_pattens = patterns[sorel_idx];

        let bins: Vec<usize> = match_patterns(&words, &sorel_pattens)
            .iter()
            .map(|x| x.to_owned())
            .filter(|&x| x != 0)
            .collect();

        let bh: HashSet<usize> = HashSet::from_iter(bins);
        let eh: HashSet<usize> = HashSet::from_iter(KNOWN_BINS);

        assert_eq!(bh, eh)
    }

    #[test]
    fn known_bins_simd() {
        let words = all_words();
        let patterns = all_patterns(&words);
        let simd_patterns = all_simd_patterns(&patterns);

        let sorel_idx = words
            .iter()
            .enumerate()
            .find(|(_, w)| w.to_str() == KNOWN_WORD)
            .unwrap()
            .0;
        let sorel_pattens = simd_patterns[sorel_idx];

        let bins: Vec<usize> = match_patterns_simd(&words, &sorel_pattens)
            .iter()
            .map(|x| x.to_owned())
            .filter(|&x| x != 0)
            .collect();

        let bh: HashSet<usize> = HashSet::from_iter(bins);
        let eh: HashSet<usize> = HashSet::from_iter(KNOWN_BINS);

        assert_eq!(bh, eh)
    }
}
