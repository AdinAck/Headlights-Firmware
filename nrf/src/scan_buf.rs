#[cfg(feature = "defmt")]
use defmt::Format;

use heapless::Deque;

#[cfg_attr(feature = "defmt", derive(Format))]
#[cfg_attr(not(feature = "defmt"), derive(Debug))]
pub enum Error {
    Full,
}

pub struct ScanBuf<const N: usize> {
    queue: Deque<u8, N>,
}

#[allow(unused)]
impl<const N: usize> ScanBuf<N> {
    pub const fn new() -> Self {
        Self {
            queue: Deque::new(),
        }
    }

    pub fn push_slice(&mut self, slice: &[u8]) -> Result<(), Error> {
        for &b in slice {
            self.queue.push_back(b).map_err(|_| Error::Full)?
        }

        Ok(())
    }

    pub fn eat(&mut self, n: usize) {
        for _ in 0..n {
            if self.queue.pop_front() == None {
                return;
            }
        }
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }

    pub fn inner(&self) -> &Deque<u8, N> {
        &self.queue
    }
}
