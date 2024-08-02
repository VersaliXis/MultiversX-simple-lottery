#![no_std]

#[allow(unused_imports)]
use multiversx_sc::imports::*;

/// An empty contract. To be used as a template when starting a new contract from scratch.
#[multiversx_sc::contract]
pub trait RandomPicker {

    #[endpoint(randomPickIndex)]
    fn random_pick_index(&self, max_index: u32) -> usize{
        let mut rand_source = RandomnessSource::new();
        let random_ticket_index = rand_source.next_usize_in_range(0, max_index.try_into().unwrap()); //upper bound not included
        random_ticket_index
    }
    
    #[init]
    fn init(&self) {}

    #[upgrade]
    fn upgrade(&self) {}
}
