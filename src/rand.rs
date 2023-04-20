use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use std::cell::RefCell;

thread_local! {
    pub static THREAD_LOCAL_SMALL_RNG: RefCell<SmallRng> = RefCell::new(SmallRng::from_entropy());
}

pub fn next_insecure_rand_u64() -> u64 {
    THREAD_LOCAL_SMALL_RNG.with(|cell| cell.borrow_mut().next_u64())
}
