use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) type Serial = usize;

/// Serial generator
pub(crate) struct SerialGenerator {
    serial: AtomicUsize,
}

impl SerialGenerator {
    pub(crate) fn new(initial: Serial) -> Self {
        Self {
            serial: AtomicUsize::new(initial + 1),
        }
    }

    pub(crate) fn gen(&self) -> Serial {
        self.serial.fetch_add(1, Ordering::Relaxed)
    }
}
