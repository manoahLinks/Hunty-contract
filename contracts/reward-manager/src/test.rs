#[cfg(test)]
mod test {
    use crate::storage::Storage;
    use crate::RewardManager;
    use soroban_sdk::testutils::{Address as _, Ledger as _};
    use soroban_sdk::{token, Address, Env};

    /// Registers the RewardManager contract and a mock SAC token.
    /// Returns (contract_id, token_address, token_admin).
    fn setup(env: &Env) -> (Address, Address, Address) {
        let contract_id = env.register(RewardManager, ());
        let token_admin = Address::generate(env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_contract.address();
        (contract_id, token_address, token_admin)
    }

    /// Mints tokens to an address using the SAC admin.
    fn mint_tokens(env: &Env, token_address: &Address, admin: &Address, to: &Address, amount: i128) {
        let client = token::StellarAssetClient::new(env, token_address);
        client.mint(to, &amount);
    }

    fn get_balance(env: &Env, token_address: &Address, addr: &Address) -> i128 {
        let client = token::Client::new(env, token_address);
        client.balance(addr)
    }

    #[test]
    fn test_initialize_sets_xlm_token() {
        let env = Env::default();
        let (contract_id, token_address, _) = setup(&env);

        env.as_contract(&contract_id, || {
            RewardManager::initialize(env.clone(), token_address.clone());
            assert_eq!(Storage::get_xlm_token(&env), Some(token_address.clone()));
        });
    }

    #[test]
    fn test_fund_reward_pool() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, token_address, token_admin) = setup(&env);
        let funder = Address::generate(&env);

        // Mint tokens to funder
        mint_tokens(&env, &token_address, &token_admin, &funder, 10_000);

        env.as_contract(&contract_id, || {
            RewardManager::initialize(env.clone(), token_address.clone());
            RewardManager::fund_reward_pool(env.clone(), funder.clone(), 1, 5_000).unwrap();
        });

        // Verify pool balance
        env.as_contract(&contract_id, || {
            assert_eq!(RewardManager::get_pool_balance(env.clone(), 1), 5_000);
        });

        // Verify tokens transferred to contract
        assert_eq!(get_balance(&env, &token_address, &contract_id), 5_000);
        assert_eq!(get_balance(&env, &token_address, &funder), 5_000);
    }

    #[test]
    fn test_fund_reward_pool_invalid_amount() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, token_address, _) = setup(&env);
        let funder = Address::generate(&env);

        env.as_contract(&contract_id, || {
            RewardManager::initialize(env.clone(), token_address.clone());
            let result = RewardManager::fund_reward_pool(env.clone(), funder.clone(), 1, 0);
            assert_eq!(result, Err(crate::errors::RewardErrorCode::InvalidAmount));
        });
    }

    #[test]
    fn test_fund_reward_pool_not_initialized() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, _, _) = setup(&env);
        let funder = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let result = RewardManager::fund_reward_pool(env.clone(), funder.clone(), 1, 1000);
            assert_eq!(result, Err(crate::errors::RewardErrorCode::NotInitialized));
        });
    }

    #[test]
    fn test_distribute_rewards_success() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, token_address, token_admin) = setup(&env);
        let funder = Address::generate(&env);
        let player = Address::generate(&env);

        // Mint and fund
        mint_tokens(&env, &token_address, &token_admin, &funder, 10_000);

        env.as_contract(&contract_id, || {
            RewardManager::initialize(env.clone(), token_address.clone());
            RewardManager::fund_reward_pool(env.clone(), funder.clone(), 1, 5_000).unwrap();

            let result = RewardManager::distribute_rewards(
                env.clone(),
                player.clone(),
                1,
                2_000,
                false,
            );
            assert!(result);
        });

        // Verify player received tokens
        assert_eq!(get_balance(&env, &token_address, &player), 2_000);
        // Verify contract balance decreased
        assert_eq!(get_balance(&env, &token_address, &contract_id), 3_000);

        // Verify pool balance updated
        env.as_contract(&contract_id, || {
            assert_eq!(RewardManager::get_pool_balance(env.clone(), 1), 3_000);
        });

        // Verify distribution tracked
        env.as_contract(&contract_id, || {
            assert!(RewardManager::is_reward_distributed(
                env.clone(),
                1,
                player.clone()
            ));
        });
    }

    #[test]
    fn test_distribute_rewards_insufficient_pool() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, token_address, token_admin) = setup(&env);
        let funder = Address::generate(&env);
        let player = Address::generate(&env);

        mint_tokens(&env, &token_address, &token_admin, &funder, 1_000);

        env.as_contract(&contract_id, || {
            RewardManager::initialize(env.clone(), token_address.clone());
            RewardManager::fund_reward_pool(env.clone(), funder.clone(), 1, 1_000).unwrap();

            // Try to distribute more than pool has
            let result = RewardManager::distribute_rewards(
                env.clone(),
                player.clone(),
                1,
                5_000,
                false,
            );
            assert!(!result);
        });

        // Verify player didn't receive tokens
        assert_eq!(get_balance(&env, &token_address, &player), 0);
    }

    #[test]
    fn test_distribute_rewards_double_distribution() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, token_address, token_admin) = setup(&env);
        let funder = Address::generate(&env);
        let player = Address::generate(&env);

        mint_tokens(&env, &token_address, &token_admin, &funder, 10_000);

        env.as_contract(&contract_id, || {
            RewardManager::initialize(env.clone(), token_address.clone());
            RewardManager::fund_reward_pool(env.clone(), funder.clone(), 1, 10_000).unwrap();

            // First distribution — success
            let result1 = RewardManager::distribute_rewards(
                env.clone(),
                player.clone(),
                1,
                2_000,
                false,
            );
            assert!(result1);

            // Second distribution — blocked
            let result2 = RewardManager::distribute_rewards(
                env.clone(),
                player.clone(),
                1,
                2_000,
                false,
            );
            assert!(!result2);
        });

        // Verify player only received once
        assert_eq!(get_balance(&env, &token_address, &player), 2_000);
    }

    #[test]
    fn test_distribute_rewards_invalid_amount() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, token_address, _) = setup(&env);
        let player = Address::generate(&env);

        env.as_contract(&contract_id, || {
            RewardManager::initialize(env.clone(), token_address.clone());

            let result = RewardManager::distribute_rewards(
                env.clone(),
                player.clone(),
                1,
                0,
                false,
            );
            assert!(!result);
        });
    }

    #[test]
    fn test_distribute_rewards_not_initialized() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, _, _) = setup(&env);
        let player = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let result = RewardManager::distribute_rewards(
                env.clone(),
                player.clone(),
                1,
                1_000,
                false,
            );
            assert!(!result);
        });
    }

    #[test]
    fn test_distribute_rewards_multiple_players() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, token_address, token_admin) = setup(&env);
        let funder = Address::generate(&env);
        let player1 = Address::generate(&env);
        let player2 = Address::generate(&env);
        let player3 = Address::generate(&env);

        mint_tokens(&env, &token_address, &token_admin, &funder, 30_000);

        env.as_contract(&contract_id, || {
            RewardManager::initialize(env.clone(), token_address.clone());
            RewardManager::fund_reward_pool(env.clone(), funder.clone(), 1, 30_000).unwrap();

            assert!(RewardManager::distribute_rewards(
                env.clone(), player1.clone(), 1, 10_000, false
            ));
            assert!(RewardManager::distribute_rewards(
                env.clone(), player2.clone(), 1, 10_000, false
            ));
            assert!(RewardManager::distribute_rewards(
                env.clone(), player3.clone(), 1, 10_000, false
            ));
        });

        assert_eq!(get_balance(&env, &token_address, &player1), 10_000);
        assert_eq!(get_balance(&env, &token_address, &player2), 10_000);
        assert_eq!(get_balance(&env, &token_address, &player3), 10_000);
        assert_eq!(get_balance(&env, &token_address, &contract_id), 0);

        env.as_contract(&contract_id, || {
            assert_eq!(RewardManager::get_pool_balance(env.clone(), 1), 0);
        });
    }

    #[test]
    fn test_get_pool_balance_after_fund_and_distribute() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, token_address, token_admin) = setup(&env);
        let funder = Address::generate(&env);
        let player = Address::generate(&env);

        mint_tokens(&env, &token_address, &token_admin, &funder, 10_000);

        env.as_contract(&contract_id, || {
            RewardManager::initialize(env.clone(), token_address.clone());

            // Initially zero
            assert_eq!(RewardManager::get_pool_balance(env.clone(), 1), 0);

            // After funding
            RewardManager::fund_reward_pool(env.clone(), funder.clone(), 1, 8_000).unwrap();
            assert_eq!(RewardManager::get_pool_balance(env.clone(), 1), 8_000);

            // After distribution
            RewardManager::distribute_rewards(env.clone(), player.clone(), 1, 3_000, false);
            assert_eq!(RewardManager::get_pool_balance(env.clone(), 1), 5_000);
        });
    }

    #[test]
    fn test_fund_reward_pool_additive() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, token_address, token_admin) = setup(&env);
        let funder = Address::generate(&env);

        mint_tokens(&env, &token_address, &token_admin, &funder, 20_000);

        env.as_contract(&contract_id, || {
            RewardManager::initialize(env.clone(), token_address.clone());
            RewardManager::fund_reward_pool(env.clone(), funder.clone(), 1, 5_000).unwrap();
        });

        env.mock_all_auths();
        env.as_contract(&contract_id, || {
            RewardManager::fund_reward_pool(env.clone(), funder.clone(), 1, 3_000).unwrap();
            assert_eq!(RewardManager::get_pool_balance(env.clone(), 1), 8_000);
        });

        assert_eq!(get_balance(&env, &token_address, &contract_id), 8_000);
    }

    #[test]
    fn test_separate_hunt_pools() {
        let env = Env::default();
        env.mock_all_auths();
        let (contract_id, token_address, token_admin) = setup(&env);
        let funder = Address::generate(&env);
        let player = Address::generate(&env);

        mint_tokens(&env, &token_address, &token_admin, &funder, 20_000);

        // Initialize and fund hunt 1
        env.as_contract(&contract_id, || {
            RewardManager::initialize(env.clone(), token_address.clone());
            RewardManager::fund_reward_pool(env.clone(), funder.clone(), 1, 5_000).unwrap();
        });

        // Fund hunt 2
        env.mock_all_auths();
        env.as_contract(&contract_id, || {
            RewardManager::fund_reward_pool(env.clone(), funder.clone(), 2, 10_000).unwrap();
        });

        // Verify pools are separate
        env.as_contract(&contract_id, || {
            assert_eq!(RewardManager::get_pool_balance(env.clone(), 1), 5_000);
            assert_eq!(RewardManager::get_pool_balance(env.clone(), 2), 10_000);
        });

        // Distribute from hunt 1
        env.mock_all_auths();
        env.as_contract(&contract_id, || {
            assert!(RewardManager::distribute_rewards(
                env.clone(), player.clone(), 1, 3_000, false
            ));
            assert_eq!(RewardManager::get_pool_balance(env.clone(), 1), 2_000);
            assert_eq!(RewardManager::get_pool_balance(env.clone(), 2), 10_000);
        });

        // Player can still claim from hunt 2
        env.mock_all_auths();
        env.as_contract(&contract_id, || {
            assert!(RewardManager::distribute_rewards(
                env.clone(), player.clone(), 2, 5_000, false
            ));
            assert_eq!(RewardManager::get_pool_balance(env.clone(), 2), 5_000);
        });
    }
}
