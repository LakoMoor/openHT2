//! A flat 32-bit address space, so ported code can use the same addresses and field offsets as
//! the original and be compared byte for byte against it.
//!
//! Unwritten memory reads as zero. `alloc` is a bump allocator starting at [`HEAP_BASE`]
//! (16-byte aligned, never freed), which the emulator-side tools mirror.

use std::collections::HashMap;

pub const HEAP_BASE: u32 = 0x2800_0000;
const PAGE: u32 = 4096;

#[derive(Default, Clone)]
pub struct Mem {
    pages: HashMap<u32, Box<[u8; PAGE as usize]>>,
    heap_next: u32,
}

impl Mem {
    pub fn new() -> Self {
        Mem { pages: HashMap::new(), heap_next: HEAP_BASE }
    }

    pub fn rd8(&self, a: u32) -> u8 {
        self.pages.get(&(a / PAGE)).map_or(0, |p| p[(a % PAGE) as usize])
    }

    pub fn wr8(&mut self, a: u32, v: u8) {
        self.pages.entry(a / PAGE).or_insert_with(|| Box::new([0; PAGE as usize]))[(a % PAGE) as usize] = v;
    }

    pub fn read(&self, a: u32, n: usize) -> Vec<u8> {
        (0..n as u32).map(|i| self.rd8(a.wrapping_add(i))).collect()
    }

    pub fn write(&mut self, a: u32, bytes: &[u8]) {
        for (i, b) in bytes.iter().enumerate() {
            self.wr8(a.wrapping_add(i as u32), *b);
        }
    }

    pub fn u16(&self, a: u32) -> u16 {
        u16::from_le_bytes(self.read(a, 2).try_into().unwrap())
    }

    pub fn i16(&self, a: u32) -> i16 {
        self.u16(a) as i16
    }

    pub fn u32(&self, a: u32) -> u32 {
        u32::from_le_bytes(self.read(a, 4).try_into().unwrap())
    }

    pub fn i32(&self, a: u32) -> i32 {
        self.u32(a) as i32
    }

    pub fn f32(&self, a: u32) -> f32 {
        f32::from_bits(self.u32(a))
    }

    pub fn f64(&self, a: u32) -> f64 {
        f64::from_le_bytes(self.read(a, 8).try_into().unwrap())
    }

    pub fn set_u16(&mut self, a: u32, v: u16) {
        self.write(a, &v.to_le_bytes());
    }

    pub fn set_i16(&mut self, a: u32, v: i16) {
        self.write(a, &v.to_le_bytes());
    }

    pub fn set_u32(&mut self, a: u32, v: u32) {
        self.write(a, &v.to_le_bytes());
    }

    pub fn set_i32(&mut self, a: u32, v: i32) {
        self.write(a, &v.to_le_bytes());
    }

    pub fn set_f32(&mut self, a: u32, v: f32) {
        self.write(a, &v.to_le_bytes());
    }

    pub fn set_f64(&mut self, a: u32, v: f64) {
        self.write(a, &v.to_le_bytes());
    }

    /// Continue allocating from `addr` (for reproducing the addresses of an existing run).
    pub fn set_heap_next(&mut self, addr: u32) {
        self.heap_next = addr;
    }

    /// Allocate `n` zeroed bytes (the original `malloc`; memory is never reused).
    pub fn alloc(&mut self, n: u32) -> u32 {
        let a = self.heap_next;
        self.heap_next = self.heap_next.wrapping_add((n + 15) & !15);
        a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_zero_and_round_trips() {
        let mut m = Mem::new();
        assert_eq!(m.u32(0x1234_5678), 0);
        m.set_f32(0x0FFE, 1.5); // straddles a page boundary
        assert_eq!(m.f32(0x0FFE), 1.5);
        m.set_f64(0x2000, -2.25);
        assert_eq!(m.f64(0x2000), -2.25);
        m.set_i16(0x3000, -7);
        assert_eq!(m.i16(0x3000), -7);
    }

    #[test]
    fn allocator_is_sequential_and_aligned() {
        let mut m = Mem::new();
        let a = m.alloc(20);
        let b = m.alloc(4);
        assert_eq!(a, HEAP_BASE);
        assert_eq!(b, HEAP_BASE + 32);
    }
}
