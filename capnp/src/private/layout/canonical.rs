//! Check canonical preorder using word indices into a single borrowed segment.
#![forbid(unsafe_code)]

use super::{ElementSize, WirePointer, WirePointerKind, data_bits_per_element};
use crate::private::primitive::WireValue;
use crate::private::read_limiter::ReadLimiter;
use crate::private::units::{BITS_PER_WORD, BYTES_PER_WORD};
use crate::{Error, ErrorKind, Result};

pub(crate) fn check(segment: &[u8], limiter: &ReadLimiter, nesting_limit: i32) -> Result<bool> {
    let mut walk = Walk {
        segment,
        limiter,
        next_word: 1,
    };
    walk.check_bounds(0, 1)?;
    Ok(walk.pointer(0, nesting_limit)? && walk.next_word == segment.len() / BYTES_PER_WORD)
}

struct Walk<'a> {
    segment: &'a [u8],
    limiter: &'a ReadLimiter,
    // The next object in preorder must start at this word.
    next_word: usize,
}

impl Walk<'_> {
    fn word(&self, index: usize) -> &[u8; BYTES_PER_WORD] {
        self.segment[index * BYTES_PER_WORD..]
            .first_chunk()
            .unwrap()
    }

    fn wire_pointer(&self, index: usize) -> WirePointer {
        let (low, high) = self.word(index).split_at(4);
        WirePointer {
            offset_and_kind: WireValue::new(u32::from_le_bytes(low.try_into().unwrap())),
            upper32bits: WireValue::new(u32::from_le_bytes(high.try_into().unwrap())),
        }
    }

    fn check_bounds(&self, start: usize, words: usize) -> Result<()> {
        let segment_words = self.segment.len() / BYTES_PER_WORD;
        if start > segment_words || words > segment_words - start {
            return Err(Error::from_kind(
                ErrorKind::MessageContainsOutOfBoundsPointer,
            ));
        }
        self.limiter.can_read(words)
    }

    // Callers check bounds and charge the read even if the object is out of preorder.
    fn consume(&mut self, start: usize, words: usize) -> bool {
        if start != self.next_word {
            return false;
        }
        self.next_word += words;
        true
    }

    fn pointer(&mut self, index: usize, nesting_limit: i32) -> Result<bool> {
        let wire = self.wire_pointer(index);
        if wire.is_null() {
            return Ok(true);
        }
        if !wire.is_positional() {
            return Ok(false);
        }
        let target = index
            .checked_add_signed(wire.offset_in_words() as isize)
            .filter(|&word| word <= self.segment.len() / BYTES_PER_WORD)
            .ok_or_else(|| Error::from_kind(ErrorKind::MessageContainsOutOfBoundsPointer))?;
        match wire.kind() {
            WirePointerKind::Struct => {
                if nesting_limit <= 0 {
                    return Err(Error::from_kind(
                        ErrorKind::MessageIsTooDeeplyNestedOrContainsCycles,
                    ));
                }
                let words = wire.struct_word_size() as usize;
                self.check_bounds(target, words)?;
                if words == 0 {
                    return Ok(target == index);
                }
                if !self.consume(target, words) {
                    return Ok(false);
                }
                let pointers = target + usize::from(wire.struct_data_size());
                let end = target + words;
                let truncated = self.section_is_truncated(target, pointers)
                    && self.section_is_truncated(pointers, end);
                Ok(self.pointers(pointers, end, nesting_limit - 1)? && truncated)
            }
            WirePointerKind::List => {
                if nesting_limit <= 0 {
                    return Err(Error::from_kind(ErrorKind::NestingLimitExceeded));
                }
                self.list(&wire, target, nesting_limit - 1)
            }
            _ => unreachable!(), // Only struct and list pointers are positional.
        }
    }

    fn section_is_truncated(&self, start: usize, end: usize) -> bool {
        start == end || *self.word(end - 1) != [0; BYTES_PER_WORD]
    }

    fn pointers(&mut self, start: usize, end: usize, nesting_limit: i32) -> Result<bool> {
        for index in start..end {
            if !self.pointer(index, nesting_limit)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn list(&mut self, wire: &WirePointer, target: usize, nesting_limit: i32) -> Result<bool> {
        let size = wire.list_element_size();
        if size == ElementSize::InlineComposite {
            return self.struct_list(
                target,
                wire.list_inline_composite_word_count(),
                nesting_limit,
            );
        }
        let count = wire.list_element_count() as usize;
        let bits_per_element = if size == ElementSize::Pointer {
            BITS_PER_WORD as u64
        } else {
            u64::from(data_bits_per_element(size))
        };
        let bits = bits_per_element * count as u64;
        let words = super::wire_helpers::round_bits_up_to_words(bits) as usize;
        self.check_bounds(target, words)?;
        if size == ElementSize::Void {
            self.limiter.can_read(count)?;
        }
        if !self.consume(target, words) {
            return Ok(false);
        }
        if size == ElementSize::Pointer {
            return self.pointers(target, target + words, nesting_limit);
        }
        let used_bits = bits % BITS_PER_WORD as u64;
        Ok(used_bits == 0 || u64::from_le_bytes(*self.word(target + words - 1)) >> used_bits == 0)
    }

    fn struct_list(&mut self, target: usize, word_count: u32, nesting_limit: i32) -> Result<bool> {
        // The word count in the list pointer excludes the tag.
        let words_with_tag = 1 + word_count as usize;
        self.check_bounds(target, words_with_tag)?;
        let tag = self.wire_pointer(target);
        let element_words = tag.check_inline_composite_tag(word_count)?;
        let stride = tag.struct_word_size() as usize;
        let count = tag.inline_composite_list_element_count() as usize;
        if stride == 0 {
            self.limiter.can_read(count)?;
        }
        if !self.consume(target, words_with_tag) || element_words != word_count {
            return Ok(false);
        }
        if stride == 0 {
            return Ok(true);
        }
        // All element bodies precede their children. Each of the two sections
        // must use its final word in at least one element.
        let mut data_truncated = false;
        let mut pointers_truncated = false;
        for i in 0..count {
            let start = target + 1 + i * stride;
            let pointers = start + usize::from(tag.struct_data_size());
            let end = start + stride;
            data_truncated |= self.section_is_truncated(start, pointers);
            pointers_truncated |= self.section_is_truncated(pointers, end);
            // Reaching a pointer through a struct element uses one more nesting level.
            if !self.pointers(pointers, end, nesting_limit - 1)? {
                return Ok(false);
            }
        }
        Ok(data_truncated && pointers_truncated)
    }
}
