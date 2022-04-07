#![cfg_attr(
    target_arch = "spirv",
    feature(register_attr),
    register_attr(spirv),
    no_std
)]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
#![deny(warnings)]

extern crate spirv_std;

use crate::spirv_std::num_traits::Float;
use glam::UVec3;
use spirv_std::glam;
#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

#[derive(Default, Copy, Clone)]
#[repr(transparent)]
pub struct AsciiBitSet {
    pub set: u32,
}

impl AsciiBitSet {
    #[inline]
    pub fn is_superset(&self, set: &Self) -> bool {
        self.set & set.set ^ set.set == 0
    }

    #[inline]
    pub fn is_disjoint(&self, set: &Self) -> bool {
        self.set & set.set == 0
    }
}

#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct Fivegram {
    pub word: u32,
    pub letter_mask: u32,
}

impl Fivegram {
    #[inline]
    pub fn exact_match(&self, pattern: &Self) -> bool {
        self.word & pattern.letter_mask ^ pattern.word == 0
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
}

#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct Word {
    pub fivegram: Fivegram,
    pub letters: AsciiBitSet,
}

pub const FIVEGRAM: usize = 5;
pub const PATTERN_COUNT: usize = 3usize.pow(FIVEGRAM as u32);
pub type Patterns = [Pattern; PATTERN_COUNT];

#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct Pattern {
    pub match_word: Fivegram,
    pub present_letter: AsciiBitSet,
    pub absent_word: Fivegram,
    pub absent_letter: AsciiBitSet,
}

impl Pattern {
    #[inline]
    pub fn matches_word(&self, word: &Word) -> bool {
        word.letters.is_superset(&self.present_letter)
            && word.letters.is_disjoint(&self.absent_letter)
            && word.fivegram.exact_match(&self.match_word)
            && !word.fivegram.any_pos_match(&self.absent_word)
    }
}

const WORD_COUNT: usize = 12972;

// LocalSize/numthreads of (x = 64, y = 1, z = 1)
#[spirv(compute(threads(64)))]
pub fn main(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] entropies: &mut [f32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] words: &[Word],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] patterns: &[Patterns],
) {
    let idx = id.x as usize;

    let mut entropy = 0f32;
    for i in 0..PATTERN_COUNT {
        let mut matches = 0f32;

        for j in 0..WORD_COUNT {
            if patterns[idx][i].matches_word(&words[j]) {
                matches += 1.0;
            }
        }

        if matches != 0.0 {
            let p = matches / WORD_COUNT as f32;
            entropy += -p * p.log2();
        }
    }

    entropies[idx] = entropy;
}
