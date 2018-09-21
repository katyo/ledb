use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) type Serial = usize;

pub(crate) trait Enumerable {
    fn enumerate(&mut self, new_serial: Serial);
}

/// Serial generator
pub(crate) struct SerialGenerator {
    serial: AtomicUsize,
}

impl SerialGenerator {
    pub(crate) fn new() -> Self {
        Self {
            serial: AtomicUsize::new(0),
        }
    }

    pub(crate) fn set(&self, value: Serial) {
        self.serial.store(value, Ordering::SeqCst);
    }

    pub(crate) fn gen(&self) -> Serial {
        self.serial.fetch_add(1, Ordering::SeqCst)
    }

    pub(crate) fn enumerate<E: Enumerable>(&self, mut data: E) -> E {
        data.enumerate(self.gen());
        data
    }
}
