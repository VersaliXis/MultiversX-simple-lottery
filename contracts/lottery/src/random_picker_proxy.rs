#![no_std]

use multiversx_sc::imports::*;


#[multiversx_sc::proxy]
pub trait RandomPicker {
    #[endpoint(randomPickIndex)]
    fn random_pick_index(&self, max_index: u32) -> usize;
}