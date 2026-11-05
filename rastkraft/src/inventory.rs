//! Инвентарь со стопками: 9 ячеек быстрого доступа + 27 основных.

use crate::items::Item;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Stack {
    pub item: Item,
    pub count: u16,
    /// Зачарование: вид × 4 + уровень (0 — нет).
    pub ench: u8,
}

impl Stack {
    pub const EMPTY: Stack = Stack { item: Item::Empty, count: 0, ench: 0 };

    pub fn new(item: Item, count: u16) -> Self {
        if item == Item::Empty || count == 0 { Stack::EMPTY } else { Stack { item, count, ench: 0 } }
    }

    pub fn is_empty(&self) -> bool {
        self.item == Item::Empty || self.count == 0
    }
}

pub const SLOTS: usize = 36;

#[derive(Clone)]
pub struct Inventory {
    pub slots: [Stack; SLOTS],
}

impl Inventory {
    pub fn new() -> Self {
        Inventory { slots: [Stack::EMPTY; SLOTS] }
    }

    /// Добавить предметы; возвращает, сколько не поместилось.
    pub fn add(&mut self, item: Item, mut count: u16) -> u16 {
        if item == Item::Empty {
            return 0;
        }
        let max = item.max_stack().max(1);
        for s in self.slots.iter_mut() {
            if count == 0 {
                break;
            }
            if s.item == item && s.count < max {
                let n = (max - s.count).min(count);
                s.count += n;
                count -= n;
            }
        }
        for s in self.slots.iter_mut() {
            if count == 0 {
                break;
            }
            if s.is_empty() {
                let n = max.min(count);
                *s = Stack::new(item, n);
                count -= n;
            }
        }
        count
    }

    pub fn count(&self, item: Item) -> u32 {
        self.slots.iter().filter(|s| s.item == item).map(|s| s.count as u32).sum()
    }

    /// Забрать предметы (все или ничего).
    pub fn remove(&mut self, item: Item, count: u16) -> bool {
        if self.count(item) < count as u32 {
            return false;
        }
        let mut left = count;
        for s in self.slots.iter_mut().rev() {
            if left == 0 {
                break;
            }
            if s.item == item {
                let n = s.count.min(left);
                s.count -= n;
                left -= n;
                if s.count == 0 {
                    *s = Stack::EMPTY;
                }
            }
        }
        true
    }

    /// Уменьшить ячейку на 1.
    pub fn take_one(&mut self, slot: usize) -> Option<Item> {
        let s = &mut self.slots[slot];
        if s.is_empty() {
            return None;
        }
        let it = s.item;
        s.count -= 1;
        if s.count == 0 {
            *s = Stack::EMPTY;
        }
        Some(it)
    }

    /// Щелчок по ячейке с «курсорной» стопкой: взять, положить, поменять, сложить.
    pub fn click(&mut self, slot: usize, cursor: &mut Stack, right: bool) {
        let s = &mut self.slots[slot];
        if cursor.is_empty() {
            if s.is_empty() {
                return;
            }
            if right {
                let half = s.count.div_ceil(2);
                *cursor = Stack::new(s.item, half);
                s.count -= half;
                if s.count == 0 {
                    *s = Stack::EMPTY;
                }
            } else {
                *cursor = *s;
                *s = Stack::EMPTY;
            }
            return;
        }
        let max = cursor.item.max_stack();
        if s.is_empty() {
            let n = if right { 1 } else { cursor.count };
            *s = Stack::new(cursor.item, n);
            cursor.count -= n;
        } else if s.item == cursor.item && s.count < max {
            let n = if right { 1 } else { (max - s.count).min(cursor.count) };
            s.count += n;
            cursor.count -= n;
        } else if !right {
            std::mem::swap(s, cursor);
        }
        if cursor.count == 0 {
            *cursor = Stack::EMPTY;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::STONE;

    #[test]
    fn stacks_fill_and_remove() {
        let mut inv = Inventory::new();
        assert_eq!(inv.add(Item::Block(STONE), 100), 0);
        assert_eq!(inv.slots[0].count, 64);
        assert_eq!(inv.slots[1].count, 36);
        assert!(inv.remove(Item::Block(STONE), 70));
        assert_eq!(inv.count(Item::Block(STONE)), 30);
        assert!(!inv.remove(Item::Block(STONE), 31));
        let mut cur = Stack::EMPTY;
        inv.click(0, &mut cur, true);
        assert_eq!(cur.count, 15);
    }
}
