#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

use crate::errors::RewardErrorCode;
use crate::storage::Storage;
use crate::xlm_handler::XlmHandler;

#[contract]
pub struct RewardManager;

#[contractimpl]
impl RewardManager {
    /// Initializes the RewardManager with the XLM token contract address (SAC).
    /// Must be called once before any reward distribution.
    pub fn initialize(env: Env, xlm_token: Address) {
        Storage::set_xlm_token(&env, &xlm_token);
    }

    /// Funds the reward pool for a specific hunt.
    /// Transfers XLM from the funder to this contract and records the pool balance.
    ///
    /// # Arguments
    /// * `funder` - The address funding the pool (must authorize)
    /// * `hunt_id` - The hunt to fund
    /// * `amount` - XLM amount to add to the pool
    pub fn fund_reward_pool(
        env: Env,
        funder: Address,
        hunt_id: u64,
        amount: i128,
    ) -> Result<(), RewardErrorCode> {
        funder.require_auth();

        if amount <= 0 {
            return Err(RewardErrorCode::InvalidAmount);
        }

        let xlm_token = Storage::get_xlm_token(&env)
            .ok_or(RewardErrorCode::NotInitialized)?;

        // Transfer XLM from funder to this contract
        let contract_addr = env.current_contract_address();
        let client = soroban_sdk::token::Client::new(&env, &xlm_token);
        client.transfer(&funder, &contract_addr, &amount);

        // Update pool balance
        let current = Storage::get_pool_balance(&env, hunt_id);
        Storage::set_pool_balance(&env, hunt_id, current + amount);

        env.events().publish(
            (Symbol::new(&env, "PoolFunded"), hunt_id),
            (funder, amount),
        );

        Ok(())
    }

    /// Distributes XLM rewards to a player for completing a hunt.
    ///
    /// Validates sufficient pool funds, prevents double distribution,
    /// transfers XLM to the player, and updates tracking state.
    ///
    /// # Arguments
    /// * `player` - The player receiving the reward
    /// * `hunt_id` - The hunt ID
    /// * `xlm_amount` - The XLM amount to distribute
    /// * `nft_enabled` - Whether to award an NFT (reserved for future use)
    ///
    /// # Returns
    /// `true` on success
    pub fn distribute_rewards(
        env: Env,
        player: Address,
        hunt_id: u64,
        xlm_amount: i128,
        nft_enabled: bool,
    ) -> bool {
        // Validate XLM token is initialized
        let xlm_token = match Storage::get_xlm_token(&env) {
            Some(addr) => addr,
            None => return false,
        };

        // Prevent double distribution
        if Storage::is_distributed(&env, hunt_id, &player) {
            return false;
        }

        // Validate amount
        if xlm_amount <= 0 {
            // No XLM to distribute — still mark as distributed if nft_enabled
            if nft_enabled {
                // TODO: NFT distribution via NftHandler
                Storage::set_distributed(&env, hunt_id, &player);
                env.events().publish(
                    (Symbol::new(&env, "RewardDistributed"), hunt_id),
                    (player, 0i128, nft_enabled),
                );
                return true;
            }
            return false;
        }

        // Validate pool has sufficient funds
        let pool_balance = Storage::get_pool_balance(&env, hunt_id);
        if pool_balance < xlm_amount {
            return false;
        }

        // Transfer XLM to player
        let contract_addr = env.current_contract_address();
        XlmHandler::distribute_xlm(&env, &xlm_token, &contract_addr, &player, xlm_amount);

        // Update state
        Storage::set_distributed(&env, hunt_id, &player);
        Storage::set_pool_balance(&env, hunt_id, pool_balance - xlm_amount);

        // TODO: Handle NFT distribution if nft_enabled

        env.events().publish(
            (Symbol::new(&env, "RewardDistributed"), hunt_id),
            (player, xlm_amount, nft_enabled),
        );

        true
    }

    /// Returns the current reward pool balance for a hunt.
    pub fn get_pool_balance(env: Env, hunt_id: u64) -> i128 {
        Storage::get_pool_balance(&env, hunt_id)
    }

    /// Returns whether a reward has been distributed to a player for a hunt.
    pub fn is_reward_distributed(env: Env, hunt_id: u64, player: Address) -> bool {
        Storage::is_distributed(&env, hunt_id, &player)
    }
}

mod errors;
mod nft_handler;
mod storage;
mod xlm_handler;

#[cfg(test)]
mod test;
