#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, String, Vec};

// Helper to create test signers
fn create_test_signers(env: &Env, count: u32) -> Vec<BytesN<32>> {
    let mut signers = Vec::new(env);
    for i in 0..count {
        // Create deterministic test keys
        let mut key_bytes = [0u8; 32];
        key_bytes[0] = i as u8;
        signers.push_back(BytesN::from_array(env, &key_bytes));
    }
    signers
}

#[test]
fn test_initialize_success() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    assert_eq!(client.threshold(), 2);
    assert_eq!(client.signer_count(), 3);
    assert_eq!(client.nonce(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_initialize_zero_threshold() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &0); // Should fail
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_initialize_threshold_exceeds_signers() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &5); // Threshold > signers
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_initialize_empty_signers() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let empty_signers = Vec::new(&env);
    client.initialize(&empty_signers, &1);
}

#[test]
fn test_add_signer() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 2);
    client.initialize(&signers, &2);

    let new_signer = BytesN::from_array(&env, &[99u8; 32]);
    let add_type = String::from_str(&env, "add");
    let proposal_id = client.propose_signer_change(&signers.get_unchecked(0), &add_type, &new_signer, &3600);

    assert_eq!(proposal_id, 1);
    
    let proposal = client.get_signer_change_proposal(&proposal_id);
    assert_eq!(proposal.change_type, add_type);
    assert_eq!(proposal.signer, new_signer);
}

#[test]
fn test_remove_signer() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let signer_to_remove = signers.get_unchecked(2);
    let remove_type = String::from_str(&env, "remove");
    let proposal_id = client.propose_signer_change(&signers.get_unchecked(0), &remove_type, &signer_to_remove, &3600);

    assert_eq!(proposal_id, 1);
    
    let proposal = client.get_signer_change_proposal(&proposal_id);
    assert_eq!(proposal.change_type, remove_type);
    assert_eq!(proposal.signer, signer_to_remove);
}

#[test]
fn test_approve_signer_change_success() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let new_signer = BytesN::from_array(&env, &[99u8; 32]);
    let add_type = String::from_str(&env, "add");
    let proposal_id = client.propose_signer_change(&signers.get_unchecked(0), &add_type, &new_signer, &3600);

    // First approval
    client.approve_signer_change(&proposal_id, &signers.get_unchecked(1));
    
    let approvals = client.get_signer_change_approvals(&proposal_id);
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals.get_unchecked(0).signer, signers.get_unchecked(1));
}

#[test]
#[should_panic(expected = "Error(Contract, #22)")]
fn test_approve_signer_change_twice() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let new_signer = BytesN::from_array(&env, &[99u8; 32]);
    let add_type = String::from_str(&env, "add");
    let proposal_id = client.propose_signer_change(&signers.get_unchecked(0), &add_type, &new_signer, &3600);

    // First approval
    client.approve_signer_change(&proposal_id, &signers.get_unchecked(1));
    
    // Try to approve again - should fail
    client.approve_signer_change(&proposal_id, &signers.get_unchecked(1));
}

#[test]
fn test_execute_signer_change_success() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let new_signer = BytesN::from_array(&env, &[99u8; 32]);
    let add_type = String::from_str(&env, "add");
    let proposal_id = client.propose_signer_change(&signers.get_unchecked(0), &add_type, &new_signer, &3600);

    // Get threshold approvals
    client.approve_signer_change(&proposal_id, &signers.get_unchecked(1));
    client.approve_signer_change(&proposal_id, &signers.get_unchecked(2));

    // Execute the signer change
    client.execute_signer_change(&proposal_id);

    // Verify signer was added
    assert!(client.is_signer(&new_signer));
    assert_eq!(client.signer_count(), 4);
    assert!(client.is_signer_change_executed(&proposal_id));
}

#[test]
#[should_panic(expected = "Error(Contract, #23)")]
fn test_execute_signer_change_insufficient_approvals() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let new_signer = BytesN::from_array(&env, &[99u8; 32]);
    let add_type = String::from_str(&env, "add");
    let proposal_id = client.propose_signer_change(&signers.get_unchecked(0), &add_type, &new_signer, &3600);

    // Only one approval (need 2 for threshold)
    client.approve_signer_change(&proposal_id, &signers.get_unchecked(1));

    // Try to execute - should fail
    client.execute_signer_change(&proposal_id);
}

#[test]
fn test_execute_signer_change_remove_signer() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let signer_to_remove = signers.get_unchecked(2);
    let remove_type = String::from_str(&env, "remove");
    let proposal_id = client.propose_signer_change(&signers.get_unchecked(0), &remove_type, &signer_to_remove, &3600);

    // Get threshold approvals (need 2 for threshold=2)
    client.approve_signer_change(&proposal_id, &signers.get_unchecked(1));
    client.approve_signer_change(&proposal_id, &signers.get_unchecked(2));

    // Execute the signer change
    client.execute_signer_change(&proposal_id);

    // Verify signer was removed
    assert!(!client.is_signer(&signer_to_remove));
    assert_eq!(client.signer_count(), 2);
    assert!(client.is_signer_change_executed(&proposal_id));
}

#[test]
#[should_panic(expected = "Error(Contract, #20)")]
fn test_execute_signer_change_twice() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let new_signer = BytesN::from_array(&env, &[99u8; 32]);
    let add_type = String::from_str(&env, "add");
    let proposal_id = client.propose_signer_change(&signers.get_unchecked(0), &add_type, &new_signer, &3600);

    // Get threshold approvals and execute (need 2 for threshold=2)
    client.approve_signer_change(&proposal_id, &signers.get_unchecked(1));
    client.approve_signer_change(&proposal_id, &signers.get_unchecked(2));
    client.execute_signer_change(&proposal_id);

    // Try to execute again - should fail
    client.execute_signer_change(&proposal_id);
}


#[test]
fn test_create_proposal() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let proposer = signers.get_unchecked(0);
    let token_address = Address::generate(&env);
    let recipient = Address::generate(&env);
    let amount = 1000i128;
    let reason = String::from_str(&env, "Payment for services");
    let expires_in_seconds = 3600u64;

    let proposal_id = client.create_proposal(
        &proposer,
        &token_address,
        &recipient,
        &amount,
        &reason,
        &expires_in_seconds,
    );

    assert_eq!(proposal_id, 1);
    assert_eq!(client.get_proposal_count(), 1);

    let proposal = client.get_proposal(&proposal_id);
    assert_eq!(proposal.id, proposal_id);
    assert_eq!(proposal.proposer, proposer);
    assert_eq!(proposal.token_address, token_address);
    assert_eq!(proposal.recipient, recipient);
    assert_eq!(proposal.amount, amount);
    assert_eq!(proposal.reason, reason);
    assert!(!proposal.executed);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_create_proposal_unknown_proposer() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let unknown_proposer = BytesN::from_array(&env, &[99u8; 32]);
    let token_address = Address::generate(&env);
    let recipient = Address::generate(&env);
    let amount = 1000i128;
    let reason = String::from_str(&env, "Payment");

    client.create_proposal(
        &unknown_proposer,
        &token_address,
        &recipient,
        &amount,
        &reason,
        &3600u64,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #18)")]
fn test_create_proposal_invalid_amount() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let proposer = signers.get_unchecked(0);
    let token_address = Address::generate(&env);
    let recipient = Address::generate(&env);
    let invalid_amount = 0i128; // Invalid amount
    let reason = String::from_str(&env, "Payment");

    client.create_proposal(
        &proposer,
        &token_address,
        &recipient,
        &invalid_amount,
        &reason,
        &3600u64,
    ); // Should fail
}


#[test]
fn test_approve_proposal() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let proposer = signers.get_unchecked(0);
    let approver1 = signers.get_unchecked(1);
    let approver2 = signers.get_unchecked(2);

    let token_address = Address::generate(&env);
    let recipient = Address::generate(&env);
    let amount = 1000i128;
    let reason = String::from_str(&env, "Payment");

    let proposal_id = client.create_proposal(
        &proposer,
        &token_address,
        &recipient,
        &amount,
        &reason,
        &3600u64,
    );

    // First approval
    client.approve_proposal(&proposal_id, &approver1);
    let approvals = client.get_proposal_approvals(&proposal_id);
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals.get_unchecked(0).signer, approver1);

    // Second approval
    client.approve_proposal(&proposal_id, &approver2);
    let approvals = client.get_proposal_approvals(&proposal_id);
    assert_eq!(approvals.len(), 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #16)")]
fn test_approve_proposal_twice() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let proposer = signers.get_unchecked(0);
    let approver = signers.get_unchecked(1);

    let token_address = Address::generate(&env);
    let recipient = Address::generate(&env);
    let amount = 1000i128;
    let reason = String::from_str(&env, "Payment");

    let proposal_id = client.create_proposal(
        &proposer,
        &token_address,
        &recipient,
        &amount,
        &reason,
        &3600u64,
    );

    client.approve_proposal(&proposal_id, &approver);
    client.approve_proposal(&proposal_id, &approver); // Should fail
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn test_approve_nonexistent_proposal() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let approver = signers.get_unchecked(0);
    client.approve_proposal(&999u64, &approver); // Proposal does not exist
}

#[test]
fn test_revoke_approval() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let proposer = signers.get_unchecked(0);
    let approver1 = signers.get_unchecked(1);
    let approver2 = signers.get_unchecked(2);

    let token_address = Address::generate(&env);
    let recipient = Address::generate(&env);
    let amount = 1000i128;
    let reason = String::from_str(&env, "Payment");

    let proposal_id = client.create_proposal(
        &proposer,
        &token_address,
        &recipient,
        &amount,
        &reason,
        &3600u64,
    );

    // Approve by both signers
    client.approve_proposal(&proposal_id, &approver1);
    client.approve_proposal(&proposal_id, &approver2);

    let approvals = client.get_proposal_approvals(&proposal_id);
    assert_eq!(approvals.len(), 2);

    // Revoke approval from first signer
    client.revoke_approval(&proposal_id, &approver1);

    let approvals = client.get_proposal_approvals(&proposal_id);
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals.get_unchecked(0).signer, approver2);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_revoke_nonexistent_approval() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let proposer = signers.get_unchecked(0);
    let approver = signers.get_unchecked(1);

    let token_address = Address::generate(&env);
    let recipient = Address::generate(&env);
    let amount = 1000i128;
    let reason = String::from_str(&env, "Payment");

    let proposal_id = client.create_proposal(
        &proposer,
        &token_address,
        &recipient,
        &amount,
        &reason,
        &3600u64,
    );

    // Try to revoke approval that doesn't exist
    client.revoke_approval(&proposal_id, &approver); // Should fail
}

#[test]
#[should_panic(expected = "Error(Contract, #17)")]
fn test_execute_proposal_insufficient_approvals() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let proposer = signers.get_unchecked(0);
    let approver = signers.get_unchecked(1);

    let token_address = Address::generate(&env);
    let recipient = Address::generate(&env);
    let amount = 1000i128;
    let reason = String::from_str(&env, "Payment");

    let proposal_id = client.create_proposal(
        &proposer,
        &token_address,
        &recipient,
        &amount,
        &reason,
        &3600u64,
    );

    // Only one approval (threshold is 2)
    client.approve_proposal(&proposal_id, &approver);

    client.execute_proposal(&proposal_id); // Should fail - insufficient approvals
}

#[test]
#[should_panic(expected = "Error(Contract, #24)")]
fn test_create_proposal_expiry_too_short() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let token_address = Address::generate(&env);
    let recipient = Address::generate(&env);
    let reason = String::from_str(&env, "Test proposal");
    
    // Try with 30 minutes (1800 seconds) - should fail (minimum is 1 hour)
    client.create_proposal(&signers.get_unchecked(0), &token_address, &recipient, &1000, &reason, &1800);
}

#[test]
#[should_panic(expected = "Error(Contract, #24)")]
fn test_create_proposal_expiry_too_long() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let token_address = Address::generate(&env);
    let recipient = Address::generate(&env);
    let reason = String::from_str(&env, "Test proposal");
    
    // Try with 60 days (5,184,000 seconds) - should fail (maximum is 30 days)
    client.create_proposal(&signers.get_unchecked(0), &token_address, &recipient, &1000, &reason, &5_184_000);
}

#[test]
fn test_create_proposal_valid_expiry() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let token_address = Address::generate(&env);
    let recipient = Address::generate(&env);
    let reason = String::from_str(&env, "Test proposal");
    
    // Try with 2 hours (7200 seconds) - should succeed
    let proposal_id = client.create_proposal(&signers.get_unchecked(0), &token_address, &recipient, &1000, &reason, &7200);
    assert_eq!(proposal_id, 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #24)")]
fn test_propose_signer_change_expiry_too_short() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let new_signer = BytesN::from_array(&env, &[99u8; 32]);
    let add_type = String::from_str(&env, "add");
    
    // Try with 30 minutes (1800 seconds) - should fail (minimum is 1 hour)
    client.propose_signer_change(&signers.get_unchecked(0), &add_type, &new_signer, &1800);
}

#[test]
#[should_panic(expected = "Error(Contract, #24)")]
fn test_propose_signer_change_expiry_too_long() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let new_signer = BytesN::from_array(&env, &[99u8; 32]);
    let add_type = String::from_str(&env, "add");
    
    // Try with 60 days (5,184,000 seconds) - should fail (maximum is 30 days)
    client.propose_signer_change(&signers.get_unchecked(0), &add_type, &new_signer, &5_184_000);
}

#[test]
fn test_propose_signer_change_valid_expiry() {
    let env = Env::default();
    let contract_id = env.register(MultiSigContract, ());
    let client = MultiSigContractClient::new(&env, &contract_id);

    let signers = create_test_signers(&env, 3);
    client.initialize(&signers, &2);

    let new_signer = BytesN::from_array(&env, &[99u8; 32]);
    let add_type = String::from_str(&env, "add");
    
    // Try with 2 hours (7200 seconds) - should succeed
    let proposal_id = client.propose_signer_change(&signers.get_unchecked(0), &add_type, &new_signer, &7200);
    assert_eq!(proposal_id, 1);
}
