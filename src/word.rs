use crate::fivegram::FIVEGRAM;
use crate::{AsciiBitSet, Fivegram};

pub type WordBytes = [u8; FIVEGRAM];

pub fn wordbytes_from_str(s: &str) -> WordBytes {
    assert_eq!(s.len(), FIVEGRAM);

    let mut bytes = [0; FIVEGRAM];
    bytes.copy_from_slice(&s.as_bytes()[0..FIVEGRAM]);

    bytes
}

/**
Pre-computed word bit-packing as well as letter hash
**/
#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
pub struct Word {
    pub bytes: WordBytes,
    pub fivegram: Fivegram,
    pub letters: AsciiBitSet,
}

impl Word {
    pub fn to_str(&self) -> String {
        self.bytes.iter().map(|&c| char::from(c)).collect()
    }

    pub fn from_str(s: &str) -> Word {
        let bytes = wordbytes_from_str(s);

        Word {
            bytes,
            fivegram: Fivegram::from_bytes(&bytes),
            letters: AsciiBitSet::from_bytes(&bytes),
        }
    }
}
