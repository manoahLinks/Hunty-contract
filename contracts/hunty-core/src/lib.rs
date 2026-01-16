#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct HuntyCore;

#[contractimpl]
impl HuntyCore {
    // TODO: Implement hunt management functions
}

mod types;
mod storage;
mod errors;

#[cfg(test)]
mod test;

