use std::{cmp::Ordering, convert::TryFrom, fmt::Display, hash::{Hash, Hasher}, ops::{Add, Div, Mul, Sub}};

pub type MessageNumber = u24;
pub type SequencingIndex = u24;
pub type OrderingIndex = u24;
pub type OrderingChannelIndex = u8;
pub type DatagramSequenceNumber = u24;

#[allow(non_camel_case_types)]
#[derive(Default, Clone, Copy, Debug)]
pub struct u24(u32);

impl u24 {
    pub const MAX: Self = u24(0xFFFFFFu32);
    pub const MIN: Self = u24(0);

    fn mask(self) -> Self {
        u24(self.0 & 0xFFFFFFu32)
    }

    pub fn wrapping_add(self, rhs: Self) -> Self {
        u24(self.0.wrapping_add(rhs.0)).mask()
    }
    
    pub fn wrapping_sub(self, rhs: Self) -> Self {
        u24(self.0.wrapping_sub(rhs.0)).mask()
    }

    pub const fn from_le_bytes(bytes: [u8; 3]) -> u24 {
        u24(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0x00]))
    }
    
    pub const fn from_be_bytes(bytes: [u8; 3]) -> u24 {
        u24(u32::from_be_bytes([0x00, bytes[0], bytes[1], bytes[2]]))
    }

    pub const fn to_le_bytes(self) -> [u8; 3] {
        let bytes = self.0.to_le_bytes();
        [bytes[0], bytes[1], bytes[2]]
    }

    pub const fn to_be_bytes(self) -> [u8; 3] {
        let bytes = self.0.to_be_bytes();
        [bytes[1], bytes[2], bytes[3]]
    }
}

impl Add for u24 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let res = self.0 + rhs.0;
        assert!(res <= Self::MAX.into());
        u24(res)
    }
}

impl Sub for u24 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let res = self.0 - rhs.0;
        assert!(res <= Self::MAX.into());
        u24(res)
    }
}

impl Mul for u24 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let res = self.0 * rhs.0;
        assert!(res <= Self::MAX.into());
        u24(res)
    }
}

impl Div for u24 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let res = self.0 / rhs.0;
        assert!(res <= Self::MAX.into());
        u24(res)
    }
}

impl Display for u24 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TryFromIntError(pub(crate) ());

impl From<u8> for u24 {
    fn from(value: u8) -> Self {
        u24(value.into())
    }
}

impl From<u16> for u24 {
    fn from(value: u16) -> Self {
        u24(value.into())
    }
}

impl TryFrom<u32> for u24 {
    type Error = TryFromIntError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if (value & 0xFF000000u32) == 0u32 {
            Ok(u24(value))
        } else {
            Err(TryFromIntError(()))
        }
    }
}

impl TryFrom<i32> for u24 {
    type Error = TryFromIntError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if let Ok(u32_value) = u32::try_from(value) {
            if (u32_value & 0xFF000000u32) == 0u32 {
                Ok(u24(u32_value))
            } else {
                Err(TryFromIntError(()))
            }
        } else {
            Err(TryFromIntError(()))
        }
    }
}

impl From<&u24> for u24 {
    fn from(other: &u24) -> Self {
        u24(other.0)
    }
}

impl From<u24> for u32 {
    fn from(other: u24) -> Self {
        other.0
    }
}

impl PartialEq for u24 {
    fn eq(&self, other: &Self) -> bool {
        self.mask().0 == other.mask().0
    }
}

impl Eq for u24 {}

impl PartialOrd for u24 {
    fn partial_cmp(&self, other: &u24) -> Option<Ordering> {
        self.mask().0.partial_cmp(&other.mask().0)
    }
}

impl Ord for u24 {
    fn cmp(&self, other: &u24) -> Ordering {
        self.mask().0.cmp(&other.mask().0)
    }
}

impl Hash for u24 {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.mask().0.hash(h)
    }
}
