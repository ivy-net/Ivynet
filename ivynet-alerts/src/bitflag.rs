use serde::{Deserialize, Serialize};

/// Extremely simple checked bitflag implementation
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitFlag(u64);

impl BitFlag {
    pub fn new(v: u64) -> Self {
        Self(v)
    }

    pub fn try_flip_bit(&mut self, bit: usize) -> Result<(), BitflagError> {
        self.validate_bit(bit)?;
        self.0 ^= 1 << bit;
        Ok(())
    }

    pub fn try_set_bit(&mut self, bit: usize) -> Result<(), BitflagError> {
        self.validate_bit(bit)?;
        self.0 |= 1 << bit;
        Ok(())
    }

    pub fn try_unset_bit(&mut self, bit: usize) -> Result<(), BitflagError> {
        self.validate_bit(bit)?;
        self.0 &= !(1 << bit);
        Ok(())
    }

    pub fn try_get_bit(&self, bit: usize) -> Result<bool, BitflagError> {
        self.validate_bit(bit)?;
        Ok((self.0 & (1 << bit)) != 0)
    }

    pub fn set_bit_to(&mut self, bit: usize, value: bool) -> Result<(), BitflagError> {
        if value {
            self.try_set_bit(bit)
        } else {
            self.try_unset_bit(bit)
        }
    }

    fn validate_bit(&self, bit: usize) -> Result<(), BitflagError> {
        let bits = i64::BITS as usize;
        if bit >= bits {
            return Err(BitflagError::InvalidBit(bit, bits));
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn all_on(&mut self) {
        self.0 = u64::MAX
    }

    #[allow(dead_code)]
    fn all_off(&mut self) {
        self.0 = 0
    }

    pub fn zero() -> Self {
        Self(0)
    }
}

impl From<BitFlag> for u64 {
    fn from(flag: BitFlag) -> Self {
        flag.0
    }
}

impl From<u64> for BitFlag {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<BitFlag> for i64 {
    fn from(flag: BitFlag) -> Self {
        flag.0 as i64
    }
}

impl From<i64> for BitFlag {
    fn from(value: i64) -> Self {
        Self(value as u64)
    }
}

impl Default for BitFlag {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BitflagError {
    #[error("Invalid bit index: {0}, max: {1}")]
    InvalidBit(usize, usize),
}
