//! Plain data the dashboard paints from.

use std::time::Instant;

use crate::temps::TempReading;
use crate::weather::Forecast;

/// A fixed-capacity ring of samples. Never allocates after construction.
#[derive(Debug, Clone)]
pub struct RingBuffer<T> {
    data: Vec<T>,
    capacity: usize,
    /// Index of the next write.
    head: usize,
    len: usize,
}

impl<T: Copy + Default> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        RingBuffer {
            data: vec![T::default(); capacity],
            capacity,
            head: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, value: T) {
        self.data[self.head] = value;
        self.head = (self.head + 1) % self.capacity;
        self.len = (self.len + 1).min(self.capacity);
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn latest(&self) -> Option<T> {
        (self.len > 0).then(|| self.data[(self.head + self.capacity - 1) % self.capacity])
    }

    pub fn iter_oldest_first(&self) -> impl Iterator<Item = T> + '_ {
        let start = (self.head + self.capacity - self.len) % self.capacity;
        (0..self.len).map(move |i| self.data[(start + i) % self.capacity])
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Memory {
    pub used_bytes: u64,
    pub total_bytes: u64,
}

impl Memory {
    pub fn fraction(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            self.used_bytes as f32 / self.total_bytes as f32
        }
    }
}

/// Wall-clock time, local.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LocalTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    /// 0 = Sunday.
    pub weekday: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

pub struct Model {
    pub cpu: RingBuffer<f32>,
    pub mem: RingBuffer<f32>,
    pub memory: Memory,
    pub now: LocalTime,
    pub temps: Vec<TempReading>,
    pub temps_error: Option<String>,
    pub forecast: Option<Forecast>,
    pub forecast_at: Option<Instant>,
    pub weather_error: Option<String>,
}

impl Model {
    pub fn new(history: usize) -> Self {
        Model {
            cpu: RingBuffer::new(history),
            mem: RingBuffer::new(history),
            memory: Memory::default(),
            now: LocalTime::default(),
            temps: Vec::new(),
            temps_error: None,
            forecast: None,
            forecast_at: None,
            weather_error: None,
        }
    }
}
