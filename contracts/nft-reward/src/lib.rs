#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct NftReward;

#[contractimpl]
impl NftReward {
    // TODO: Implement NFT contract (can use Stellar Asset Contract or custom)
}

#[cfg(test)]
mod test;
