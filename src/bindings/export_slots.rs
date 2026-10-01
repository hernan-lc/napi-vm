use crate::{heap, value::Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ExportId {
    slot: usize,
    generation: u64,
}
struct Slot {
    generation: u64,
    value: Option<Value>,
    pin: Option<heap::RootId>,
}
#[derive(Default)]
pub(super) struct ExportSlots {
    slots: Vec<Slot>,
    free: Vec<usize>,
}
impl ExportSlots {
    #[cfg(test)]
    pub(super) fn assert_empty(&self) {
        assert!(
            self.slots
                .iter()
                .all(|slot| slot.value.is_none() && slot.pin.is_none())
        );
        assert_eq!(crate::heap::pin_count(), 0);
    }
    #[cfg(test)]
    pub(super) fn assert_reuse_safe(&mut self) {
        assert_eq!(self.slots.len(), 1);
        let stale = ExportId {
            slot: 0,
            generation: self.slots[0].generation,
        };
        let new = self.insert(Value::Number(42.));
        assert_ne!(stale, new);
        self.release(stale);
        assert!(matches!(self.get(new), Some(Value::Number(42.))));
        self.release(new);
        self.assert_empty();
    }
    pub(super) fn insert(&mut self, value: Value) -> ExportId {
        let pin = heap::add_root(value.clone());
        let slot = self.free.pop().unwrap_or_else(|| {
            self.slots.push(Slot {
                generation: 0,
                value: None,
                pin: None,
            });
            self.slots.len() - 1
        });
        let entry = &mut self.slots[slot];
        entry.generation += 1;
        entry.value = Some(value);
        entry.pin = Some(pin);
        ExportId {
            slot,
            generation: entry.generation,
        }
    }
    pub(super) fn get(&self, id: ExportId) -> Option<Value> {
        let entry = self.slots.get(id.slot)?;
        (entry.generation == id.generation)
            .then(|| entry.value.clone())
            .flatten()
    }
    pub(super) fn release(&mut self, id: ExportId) {
        let Some(entry) = self.slots.get_mut(id.slot) else {
            return;
        };
        if entry.generation != id.generation || entry.value.is_none() {
            return;
        }
        if let Some(pin) = entry.pin.take() {
            heap::remove_root(pin);
        }
        entry.value = None;
        if entry.generation < u64::MAX {
            self.free.push(id.slot);
        }
    }
}
impl Drop for ExportSlots {
    fn drop(&mut self) {
        for entry in &mut self.slots {
            if let Some(pin) = entry.pin.take() {
                heap::remove_root(pin);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_releases_do_not_alias_reused_pinned_slots() {
        let mut slots = ExportSlots::default();
        let old = slots.insert(Value::Number(1.));
        slots.release(old);
        let new = slots.insert(Value::Number(2.));
        slots.release(old);
        assert!(matches!(slots.get(new), Some(Value::Number(2.))));
        for _ in 0..10000 {
            slots.release(new);
            let id = slots.insert(Value::Undefined);
            slots.release(id);
        }
        assert_eq!(slots.slots.len(), 1);
    }
}
