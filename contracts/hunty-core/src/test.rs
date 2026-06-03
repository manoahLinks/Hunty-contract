use crate::HuntyCore;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, Env, String};

/// Helper to execute contract operations within the contract context.
/// Wraps calls with `env.as_contract()` for proper storage isolation.
fn execute_in_contract<T, F>(env: &Env, contract_id: &Address, f: F) -> T
where
    F: FnOnce(&Env) -> T,
{
    env.as_contract(contract_id, || f(env))
}

#[test]
fn test_get_hunt_statistics_mixed_completion_states() {
    let env = Env::default();
    env.ledger().set_timestamp(1_700_000_000);

    let creator = Address::generate(&env);
    let player1 = Address::generate(&env);
    let player2 = Address::generate(&env);
    let player3 = Address::generate(&env);
    let question = String::from_str(&env, "Q");
    let answer = String::from_str(&env, "a");

    // Register contract and create hunt
    let contract_id = env.register(HuntyCore, ());
    let hunt_id = execute_in_contract(&env, &contract_id, |env| {
        HuntyCore::create_hunt(
            env.clone(),
            creator.clone(),
            String::from_str(env, "Mixed Hunt"),
            String::from_str(env, "Desc"),
            None,
            None,
        )
        .unwrap()
    });

    // Add a single required clue worth 10 points and activate
    env.mock_all_auths();
    execute_in_contract(&env, &contract_id, |env| {
        HuntyCore::add_clue(
            env.clone(),
            hunt_id,
            question.clone(),
            answer.clone(),
            10,
            true,
        )
        .unwrap();
        HuntyCore::activate_hunt(env.clone(), hunt_id, creator.clone()).unwrap();
    });

    // Register three players
    env.mock_all_auths();
    execute_in_contract(&env, &contract_id, |env| {
        HuntyCore::register_player(env.clone(), hunt_id, player1.clone()).unwrap();
    });
    env.mock_all_auths();
    execute_in_contract(&env, &contract_id, |env| {
        HuntyCore::register_player(env.clone(), hunt_id, player2.clone()).unwrap();
    });
    env.mock_all_auths();
    execute_in_contract(&env, &contract_id, |env| {
        HuntyCore::register_player(env.clone(), hunt_id, player3.clone()).unwrap();
    });

    // Player1 and Player2 solve the required clue
    env.mock_all_auths();
    execute_in_contract(&env, &contract_id, |env| {
        HuntyCore::submit_answer(env.clone(), hunt_id, 1, player1.clone(), answer.clone()).unwrap();
    });
    env.mock_all_auths();
    execute_in_contract(&env, &contract_id, |env| {
        HuntyCore::submit_answer(env.clone(), hunt_id, 1, player2.clone(), answer.clone()).unwrap();
    });

    // Player3 remains incomplete (no submissions)

    // Fetch statistics and validate exact invariants
    let stats = execute_in_contract(&env, &contract_id, |env| {
        HuntyCore::get_hunt_statistics(env.clone(), hunt_id).unwrap()
    });

    // 3 players total, 2 completed -> floor(2/3*100) == 66
    assert_eq!(stats.total_players, 3);
    assert_eq!(stats.completed_count, 2);
    assert_eq!(stats.completion_rate_percent, 66);

    // Two players solved the single 10-point required clue => total 20
    // Average must be computed over all 3 participants: floor(20 / 3) == 6
    assert_eq!(stats.total_score_sum, 20);
    assert_eq!(stats.average_score, 6);
}
