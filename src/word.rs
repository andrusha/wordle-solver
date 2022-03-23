use crate::fivegram::FIVEGRAM;
use crate::{AsciiBitSet, Fivegram};

pub type WordBytes = [u8; FIVEGRAM];

pub fn wordbytes_from_str(s: &str) -> WordBytes {
    assert_eq!(s.len(), FIVEGRAM);

    let mut bytes = [0; FIVEGRAM];
    bytes.copy_from_slice(&s.as_bytes()[0..FIVEGRAM]);

    bytes
}

pub fn wordbytes_to_str(wb: &WordBytes) -> String {
    wb.iter().map(|&c| char::from(c)).collect()
}

/**
Pre-computed word bit-packing as well as letter hash
**/
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Word {
    pub fivegram: Fivegram,
    pub letters: AsciiBitSet,
}

impl Word {
    #[cfg(test)]
    pub fn from_str(s: &str) -> Word {
        let bytes = wordbytes_from_str(s);

        Word::from_wordbytes(&bytes)
    }

    pub fn from_wordbytes(wb: &WordBytes) -> Word {
        Word {
            fivegram: Fivegram::from_bytes(wb),
            letters: AsciiBitSet::from_bytes(wb),
        }
    }
}
