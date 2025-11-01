use std::ops::{
    Bound, Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
};

use crate::BitSize;

/// Inclusive start and end address range
#[doc(hidden)]
#[derive(Debug, PartialEq, Eq)]
pub struct AddressRange {
    pub start: BitSize,
    pub end: BitSize,
}

impl IntoIterator for AddressRange {
    type Item = BitSize;
    type IntoIter = RangeInclusive<BitSize>;

    fn into_iter(self) -> Self::IntoIter {
        RangeInclusive::new(self.start, self.end)
    }
}

impl RangeBounds<BitSize> for AddressRange {
    fn start_bound(&self) -> Bound<&BitSize> {
        Bound::Included(&self.start)
    }

    fn end_bound(&self) -> Bound<&BitSize> {
        Bound::Included(&self.end)
    }
}

impl From<BitSize> for AddressRange {
    fn from(value: BitSize) -> Self {
        Self {
            start: value,
            end: value,
        }
    }
}

// start..end
impl From<Range<BitSize>> for AddressRange {
    fn from(value: Range<BitSize>) -> Self {
        Self {
            start: value.start,
            end: value.end.saturating_sub(1),
        }
    }
}

// start..
impl From<RangeFrom<BitSize>> for AddressRange {
    fn from(value: RangeFrom<BitSize>) -> Self {
        Self {
            start: value.start,
            end: BitSize::MAX,
        }
    }
}

// ..
impl From<RangeFull> for AddressRange {
    fn from(_: RangeFull) -> Self {
        Self {
            start: 0,
            end: BitSize::MAX,
        }
    }
}

// start..=end
impl From<RangeInclusive<BitSize>> for AddressRange {
    fn from(value: RangeInclusive<BitSize>) -> Self {
        Self {
            start: *value.start(),
            end: *value.end(),
        }
    }
}

// ..end
impl From<RangeTo<BitSize>> for AddressRange {
    fn from(value: RangeTo<BitSize>) -> Self {
        Self {
            start: 0,
            end: value.end.saturating_sub(1),
        }
    }
}

// ..=end
impl From<RangeToInclusive<BitSize>> for AddressRange {
    fn from(value: RangeToInclusive<BitSize>) -> Self {
        Self {
            start: 0,
            end: value.end,
        }
    }
}
