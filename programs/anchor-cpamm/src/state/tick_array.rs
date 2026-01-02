use anchor_lang::prelude::*;
use crate::constants::TICK_ARRAY_SIZE;
use super::Tick;

/// TickArray holds a contiguous range of ticks.
#[account(zero_copy(unsafe))]
#[repr(C)]
pub struct TickArray {
    /// The pool this tick array belongs to
    pub pool: Pubkey,
    /// Starting tick index for this array (must be aligned)
    pub start_tick_index: i32,
    /// Padding for alignment
    pub _padding: [u8; 4],
    /// The ticks in this array
    pub ticks: [Tick; TICK_ARRAY_SIZE],
}

impl TickArray {
    pub const LEN: usize = 8 + // discriminator
        32 + // pool
        4 +  // start_tick_index
        4 +  // padding
        (Tick::LEN * TICK_ARRAY_SIZE); // ticks

    /// Get tick at a given index, checking bounds
    pub fn get_tick(&self, tick_index: i32, tick_spacing: u16) -> Option<&Tick> {
        let offset = self.tick_offset(tick_index, tick_spacing)?;
        Some(&self.ticks[offset])
    }

    /// Get mutable tick at a given index
    pub fn get_tick_mut(&mut self, tick_index: i32, tick_spacing: u16) -> Option<&mut Tick> {
        let offset = self.tick_offset(tick_index, tick_spacing)?;
        Some(&mut self.ticks[offset])
    }

    /// Calculate offset of tick_index within this array
    fn tick_offset(&self, tick_index: i32, tick_spacing: u16) -> Option<usize> {
        if tick_index < self.start_tick_index {
            return None;
        }
        let offset = ((tick_index - self.start_tick_index) / tick_spacing as i32) as usize;
        if offset >= TICK_ARRAY_SIZE {
            return None;
        }
        Some(offset)
    }

    /// Check if a tick index belongs to this array
    pub fn contains_tick(&self, tick_index: i32, tick_spacing: u16) -> bool {
        self.tick_offset(tick_index, tick_spacing).is_some()
    }

    /// Get the start tick index for a tick array containing the given tick
    pub fn get_start_tick_index(tick_index: i32, tick_spacing: u16) -> i32 {
        let ticks_in_array = (TICK_ARRAY_SIZE as i32) * (tick_spacing as i32);
        let mut start = (tick_index / ticks_in_array) * ticks_in_array;
        if tick_index < 0 && tick_index % ticks_in_array != 0 {
            start -= ticks_in_array;
        }
        start
    }
}
