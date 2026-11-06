//! The timed-event list of the engine: `0x5d32f0` (schedule) with the node pool (`0x5d2a70`, `0x5d2ac0`, pool object `0x7200c0`), the node
//! constructor (`0x5d3090`), the destructor (`0x5d30c0`, vtable `0x651eb4`), the sorted insert (`0x5d3660`) and the walker `0x5d3630`.
//! Node (24 bytes): `+0` vtable, `+4` previous, `+8` next, `+0xc` owner (priority `+0x14`, log value `+0x1c`), `+0x10` time, `+0x14`
//! sequence. List object: `+0` base time, `+0xc` head.

use crate::mem::Mem;

pub const POOL: u32 = 0x7200c0;
pub const NODE_VTABLE: u32 = 0x651eb4;

/// The engine log (stand-ins): `0x5d2e70(message, 0)` and the node report `0x5d2ba0(owner[+0x1c], node[+0x14])`.
pub trait QueueLog {
    fn message(&mut self, id: u32, arg: u32);
    fn node(&mut self, a: u32, b: u32);
}

/// `0x5d2a70` (thiscall, pool: `+0` slot size, `+4` count, `+8` base, `+0xc` map, `+0x10` last index): the next free slot after the last
/// one (wrapping); 0 when all are taken.
pub fn pool_alloc(m: &mut Mem, pool: u32) -> u32 {
    let (size, cnt, base, map, mut idx) = (m.u32(pool), m.u32(pool + 4), m.u32(pool + 8), m.u32(pool + 0xc), m.u32(pool + 0x10));
    if m.rd8(map.wrapping_add(idx)) != 0 {
        let mut left = cnt as i32;
        loop {
            if left <= 0 {
                return 0;
            }
            idx = idx.wrapping_add(1) % cnt;
            left -= 1;
            if m.rd8(map.wrapping_add(idx)) == 0 {
                break;
            }
        }
    }
    m.wr8(map.wrapping_add(idx), 1);
    m.set_u32(pool + 0x10, idx);
    size.wrapping_mul(idx).wrapping_add(base)
}

/// `0x5d2ac0(ptr)` (thiscall, pool): frees the slot; logs (`0x67fb40`) an index outside the pool, (`0x67fb24`) a slot that is free.
pub fn pool_free(m: &mut Mem, pool: u32, p: u32, log: &mut dyn QueueLog) {
    let idx = p.wrapping_sub(m.u32(pool + 8)) / m.u32(pool);
    if (idx as i32) < 0 || idx >= m.u32(pool + 4) {
        log.message(0x67fb40, 0);
    }
    let map = m.u32(pool + 0xc);
    if m.rd8(idx.wrapping_add(map)) == 0 {
        log.message(0x67fb24, 0);
    }
    m.wr8(idx.wrapping_add(map), 0);
}

/// `0x5d30c0(1)`: the scalar destructor of a node: the base vtable is set (`0x5ce630`) and the node goes back to the pool.
fn node_delete(m: &mut Mem, node: u32, log: &mut dyn QueueLog) {
    m.set_u32(node, 0x64d81c);
    pool_free(m, POOL, node, log);
}

/// `0x5d3090(queue, time)` (thiscall, node).
pub fn node_ctor(m: &mut Mem, node: u32, queue: u32, time: u32) {
    m.set_u32(node + 0xc, queue);
    m.set_u32(node + 4, 0);
    m.set_u32(node + 8, 0);
    m.set_u32(node, NODE_VTABLE);
    m.set_u32(node + 0x10, time);
    m.set_u32(node + 0x14, 0);
}

/// `0x5d3630` (thiscall, list): reports every node of the list (`0x5d2ba0(owner[+0x1c], node[+0x14])`).
pub fn list_report(m: &mut Mem, list: u32, log: &mut dyn QueueLog) {
    let mut n = m.u32(list + 0xc);
    while n != 0 {
        let owner = m.u32(n + 0xc);
        let (a, b) = (m.u32(owner + 0x1c), m.u32(n + 0x14));
        log.node(a, b);
        n = m.u32(n + 8);
    }
}

/// `0x5d3660(node)` (thiscall, list): adds the list's base time to the node time and inserts it by time (then by owner priority `+0x14`, later
/// ones after earlier ones of equal time and not lower priority); a node equal to a listed one (owner, time, sequence) is deleted instead.
/// No owner: ignored.
pub fn list_insert(m: &mut Mem, list: u32, node: u32, log: &mut dyn QueueLog) {
    if node == 0 || m.u32(node + 0xc) == 0 {
        return;
    }
    let t = m.u32(node + 0x10).wrapping_add(m.u32(list));
    m.set_u32(node + 0x10, t);
    let head = m.u32(list + 0xc);
    if head == 0 {
        m.set_u32(list + 0xc, node);
        return;
    }
    let mut c = head;
    while c != 0 {
        if m.u32(c + 0xc) == m.u32(node + 0xc) && m.u32(c + 0x10) == m.u32(node + 0x10) && m.u32(c + 0x14) == m.u32(node + 0x14) {
            node_delete(m, node, log);
            return;
        }
        c = m.u32(c + 8);
    }
    let next = |m: &Mem, x: u32| m.u32(x + 8);
    let prio = |m: &Mem, x: u32| m.i32(m.u32(x + 0xc) + 0x14);
    let (mut edi, nt) = (head, t as i32);
    let before = m.i32(edi + 0x10) > nt || (m.i32(edi + 0x10) == nt && prio(m, edi) < prio(m, node));
    if before {
        m.set_u32(node + 8, edi);
        m.set_u32(edi + 4, node);
        m.set_u32(list + 0xc, node);
        return;
    }
    if next(m, edi) != 0 {
        loop {
            if m.i32(next(m, edi) + 0x10) >= nt {
                break;
            }
            edi = next(m, edi);
            if next(m, edi) == 0 {
                break;
            }
        }
    }
    loop {
        if next(m, edi) == 0 {
            break;
        }
        let n = next(m, edi);
        if m.i32(n + 0x10) != nt || prio(m, n) <= prio(m, node) {
            break;
        }
        edi = n;
    }
    // 0x5d2190: link the node after `edi`
    m.set_u32(node + 4, edi);
    let nx = m.u32(edi + 8);
    m.set_u32(node + 8, nx);
    if nx != 0 {
        m.set_u32(nx + 4, node);
    }
    m.set_u32(edi + 8, node);
}

/// `0x5d32f0(time)` (thiscall, holder; list at `[+0xc]`): a node from the pool (`0x5d3060`), constructed for this holder; on failure
/// the list is reported and logged (`0x67fbfc`); then it is inserted (a null node does nothing).
pub fn schedule(m: &mut Mem, holder: u32, time: u32, log: &mut dyn QueueLog) {
    let list = m.u32(holder + 0xc);
    if list == 0 {
        return;
    }
    let mut node = pool_alloc(m, POOL);
    if node != 0 {
        node_ctor(m, node, holder, time);
    } else {
        node = 0;
    }
    if node == 0 {
        list_report(m, list, log);
        log.message(0x67fbfc, 0);
    }
    list_insert(m, list, node, log);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    struct T(Vec<String>);
    impl QueueLog for T {
        fn message(&mut self, id: u32, arg: u32) {
            self.0.push(format!("e70|{id:x}|{arg:x}"));
        }
        fn node(&mut self, a: u32, b: u32) {
            self.0.push(format!("ba0|{a:x}|{b:x}"));
        }
    }
    #[test]
    fn native_event_queue() {
        let base = 0x2a010000u32;
        let (h, pb, pm) = (base, base + 0x10000, base + 0x11000);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_event_queue.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            let blocks = [(h, 0x400usize), (pb, 0x100), (pm, 0x10), (POOL, 0x14)];
            let s0: Vec<Vec<u8>> = blocks.iter().map(|&(a, n)| m.read(a, n)).collect();
            let mut t = T(vec![]);
            schedule(&mut m, h, v[1].parse().unwrap(), &mut t);
            for (k, &(a, len)) in blocks.iter().enumerate() {
                let b = m.read(a, len);
                let d: String = (0..len)
                    .step_by(4.min(len))
                    .filter(|&i| s0[k][i..i + 4.min(len - i)] != b[i..i + 4.min(len - i)])
                    .map(|i| format!("{:x}:{}", i, b[i..i + 4.min(len - i)].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                    .collect::<Vec<_>>()
                    .join(",");
                assert_eq!(d, v[2 + k], "case {c}: block {k}");
            }
            assert_eq!(t.0.join(";"), v[6], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 500);
    }
}
