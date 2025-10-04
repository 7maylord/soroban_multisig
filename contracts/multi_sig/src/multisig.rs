use soroban_sdk::{contract, contractimpl, panic_with_error, Address, BytesN, Env, String, Vec};

use crate::errors::MultisigError;
use crate::types::{DataKey, Proposal, ProposalApproval, SignerChangeProposal, SignerChangeApproval};

#[contract]
pub struct MultiSigContract;

#[contractimpl]
impl MultiSigContract {
    pub fn initialize(env: Env, signers: Vec<BytesN<32>>, threshold: u32) {
        if env.storage().instance().has(&DataKey::Initialized) {
            panic_with_error!(&env, MultisigError::AlreadyInitialized);
        }

        if signers.len() == 0 {
            panic_with_error!(&env, MultisigError::EmptySignersList);
        }

        if threshold == 0 {
            panic_with_error!(&env, MultisigError::InvalidThreshold);
        }

        if threshold > signers.len() as u32 {
            panic_with_error!(&env, MultisigError::ThresholdExceedsSigners);
        }

        // Check for duplicate signers
        for i in 0..signers.len() {
            for j in (i + 1)..signers.len() {
                if signers.get_unchecked(i) == signers.get_unchecked(j) {
                    panic_with_error!(&env, MultisigError::DuplicateSigner);
                }
            }
        }

        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::SignerCount, &(signers.len() as u32));
        env.storage().instance().set(&DataKey::Threshold, &threshold);
        env.storage().instance().set(&DataKey::Nonce, &0u64);

        for signer in signers {
            env.storage().instance().set(&DataKey::Signer(signer.clone()), &true);
        }
    }

    pub fn propose_signer_change(
        env: Env,
        proposer: BytesN<32>,
        change_type: String,
        signer: BytesN<32>,
        expires_in_seconds: u64,
    ) -> u64 {
        Self::require_initialized(&env);
        
        // Validate expiry time (1 hour to 30 days)
        const MIN_EXPIRY_SECONDS: u64 = 3600;      // 1 hour
        const MAX_EXPIRY_SECONDS: u64 = 2_592_000; // 30 days
        
        if expires_in_seconds < MIN_EXPIRY_SECONDS {
            panic_with_error!(&env, MultisigError::InvalidExpiryTime);
        }
        
        if expires_in_seconds > MAX_EXPIRY_SECONDS {
            panic_with_error!(&env, MultisigError::InvalidExpiryTime);
        }
        
        // Verify proposer is a signer
        if !env.storage().instance().has(&DataKey::Signer(proposer.clone())) {
            panic_with_error!(&env, MultisigError::UnknownSigner);
        }

        // Validate change type
        let add_type = String::from_str(&env, "add");
        let remove_type = String::from_str(&env, "remove");
        
        if change_type != add_type && change_type != remove_type {
            panic_with_error!(&env, MultisigError::InvalidProposal);
        }

        // For add: check if signer already exists
        if change_type == add_type && env.storage().instance().has(&DataKey::Signer(signer.clone())) {
            panic_with_error!(&env, MultisigError::DuplicateSigner);
        }

        // For remove: check if signer exists
        if change_type == remove_type && !env.storage().instance().has(&DataKey::Signer(signer.clone())) {
            panic_with_error!(&env, MultisigError::SignerNotFound);
        }

        // For remove: check threshold constraint
        if change_type == remove_type {
            let current_count: u32 = env.storage().instance().get(&DataKey::SignerCount).unwrap();
            let threshold: u32 = env.storage().instance().get(&DataKey::Threshold).unwrap();
            
            if current_count - 1 < threshold {
                panic_with_error!(&env, MultisigError::ThresholdExceedsSigners);
            }
        }

        let current_time = env.ledger().timestamp();
        
        // Get next proposal ID
        let current_count: u64 = env.storage().instance()
            .get(&DataKey::ProposalCount)
            .unwrap_or(0u64);
        let proposal_id = current_count + 1;
        env.storage().instance().set(&DataKey::ProposalCount, &proposal_id);
        
        let proposal = SignerChangeProposal {
            id: proposal_id,
            proposer: proposer.clone(),
            change_type,
            signer,
            created_at: current_time,
            expires_at: current_time + expires_in_seconds,
            executed: false,
        };

        env.storage().instance().set(&DataKey::SignerChangeProposal(proposal_id), &proposal);
        
        let approvals: Vec<SignerChangeApproval> = Vec::new(&env);
        env.storage().instance().set(&DataKey::SignerChangeApprovals(proposal_id), &approvals);

        proposal_id
    }

    pub fn approve_signer_change(env: Env, proposal_id: u64, approver: BytesN<32>) {
        Self::require_initialized(&env);
        
        if !env.storage().instance().has(&DataKey::Signer(approver.clone())) {
            panic_with_error!(&env, MultisigError::UnknownSigner);
        }

        if !env.storage().instance().has(&DataKey::SignerChangeProposal(proposal_id)) {
            panic_with_error!(&env, MultisigError::SignerChangeNotFound);
        }

        if env.storage().instance().has(&DataKey::SignerChangeExecuted(proposal_id)) {
            panic_with_error!(&env, MultisigError::SignerChangeAlreadyExecuted);
        }

        let proposal: SignerChangeProposal = env.storage().instance().get(&DataKey::SignerChangeProposal(proposal_id)).unwrap();
        
        if env.ledger().timestamp() > proposal.expires_at {
            panic_with_error!(&env, MultisigError::SignerChangeExpired);
        }

        let mut approvals: Vec<SignerChangeApproval> = env.storage().instance()
            .get(&DataKey::SignerChangeApprovals(proposal_id)).unwrap_or(Vec::new(&env));

        // Check if already approved
        for i in 0..approvals.len() {
            let approval = approvals.get_unchecked(i);
            if approval.signer == approver {
                panic_with_error!(&env, MultisigError::SignerChangeAlreadyApproved);
            }
        }

        let approval = SignerChangeApproval {
            signer: approver,
            approved_at: env.ledger().timestamp(),
        };

        approvals.push_back(approval);
        env.storage().instance().set(&DataKey::SignerChangeApprovals(proposal_id), &approvals);
    }

    pub fn execute_signer_change(env: Env, proposal_id: u64) {
        Self::require_initialized(&env);
        
        if !env.storage().instance().has(&DataKey::SignerChangeProposal(proposal_id)) {
            panic_with_error!(&env, MultisigError::SignerChangeNotFound);
        }

        if env.storage().instance().has(&DataKey::SignerChangeExecuted(proposal_id)) {
            panic_with_error!(&env, MultisigError::SignerChangeAlreadyExecuted);
        }

        let proposal: SignerChangeProposal = env.storage().instance().get(&DataKey::SignerChangeProposal(proposal_id)).unwrap();
        
        if env.ledger().timestamp() > proposal.expires_at {
            panic_with_error!(&env, MultisigError::SignerChangeExpired);
        }

        let approvals: Vec<SignerChangeApproval> = env.storage().instance()
            .get(&DataKey::SignerChangeApprovals(proposal_id)).unwrap_or(Vec::new(&env));

        let threshold: u32 = env.storage().instance().get(&DataKey::Threshold).unwrap();
        
        if approvals.len() < threshold {
            panic_with_error!(&env, MultisigError::InsufficientSignerChangeApprovals);
        }

        // Execute the signer change
        let add_type = String::from_str(&env, "add");
        let remove_type = String::from_str(&env, "remove");
        
        if proposal.change_type == add_type {
            env.storage().instance().set(&DataKey::Signer(proposal.signer.clone()), &true);
            let current_count: u32 = env.storage().instance().get(&DataKey::SignerCount).unwrap();
            env.storage().instance().set(&DataKey::SignerCount, &(current_count + 1));
        } else if proposal.change_type == remove_type {
            env.storage().instance().remove(&DataKey::Signer(proposal.signer.clone()));
            let current_count: u32 = env.storage().instance().get(&DataKey::SignerCount).unwrap();
            env.storage().instance().set(&DataKey::SignerCount, &(current_count - 1));
        }

        // Mark as executed
        env.storage().instance().set(&DataKey::SignerChangeExecuted(proposal_id), &true);
        
        let mut updated_proposal = proposal;
        updated_proposal.executed = true;
        env.storage().instance().set(&DataKey::SignerChangeProposal(proposal_id), &updated_proposal);
    }

    pub fn threshold(env: Env) -> u32 {
        Self::require_initialized(&env);
        env.storage().instance().get(&DataKey::Threshold).unwrap()
    }

    pub fn signer_count(env: Env) -> u32 {
        Self::require_initialized(&env);
        env.storage().instance().get(&DataKey::SignerCount).unwrap()
    }

    pub fn nonce(env: Env) -> u64 {
        Self::require_initialized(&env);
        env.storage().instance().get(&DataKey::Nonce).unwrap()
    }

    pub fn is_signer(env: Env, signer: BytesN<32>) -> bool {
        Self::require_initialized(&env);
        env.storage().instance().has(&DataKey::Signer(signer))
    }

    fn require_initialized(env: &Env) {
        if !env.storage().instance().has(&DataKey::Initialized) {
            panic_with_error!(env, MultisigError::NotInitialized);
        }
    }

    pub fn create_proposal(
        env: Env,
        proposer: BytesN<32>,
        token_address: Address,
        recipient: Address,
        amount: i128,
        reason: String,
        expires_in_seconds: u64,
    ) -> u64 {
        Self::require_initialized(&env);
        
        // Validate expiry time (1 hour to 30 days)
        const MIN_EXPIRY_SECONDS: u64 = 3600;      // 1 hour
        const MAX_EXPIRY_SECONDS: u64 = 2_592_000; // 30 days
        
        if expires_in_seconds < MIN_EXPIRY_SECONDS {
            panic_with_error!(&env, MultisigError::InvalidExpiryTime);
        }
        
        if expires_in_seconds > MAX_EXPIRY_SECONDS {
            panic_with_error!(&env, MultisigError::InvalidExpiryTime);
        }
        
        // Verify proposer is a signer
        if !env.storage().instance().has(&DataKey::Signer(proposer.clone())) {
            panic_with_error!(&env, MultisigError::UnknownSigner);
        }

        if amount <= 0 {
            panic_with_error!(&env, MultisigError::InvalidProposal);
        }

        let current_time = env.ledger().timestamp();
        
        // Get next proposal ID directly from storage
        let current_count: u64 = env.storage().instance()
            .get(&DataKey::ProposalCount)
            .unwrap_or(0u64);
        let proposal_id = current_count + 1;
        env.storage().instance().set(&DataKey::ProposalCount, &proposal_id);
        
        let proposal = Proposal {
            id: proposal_id,
            proposer: proposer.clone(),
            token_address,
            recipient,
            amount,
            reason,
            created_at: current_time,
            expires_at: current_time + expires_in_seconds,
            executed: false,
        };

        env.storage().instance().set(&DataKey::Proposal(proposal_id), &proposal);
        
        let approvals: Vec<ProposalApproval> = Vec::new(&env);
        env.storage().instance().set(&DataKey::ProposalApprovals(proposal_id), &approvals);

        proposal_id
    }

    pub fn approve_proposal(env: Env, proposal_id: u64, approver: BytesN<32>) {
        Self::require_initialized(&env);
        
        if !env.storage().instance().has(&DataKey::Signer(approver.clone())) {
            panic_with_error!(&env, MultisigError::UnknownSigner);
        }

        if !env.storage().instance().has(&DataKey::Proposal(proposal_id)) {
            panic_with_error!(&env, MultisigError::ProposalNotFound);
        }

        if env.storage().instance().has(&DataKey::ProposalExecuted(proposal_id)) {
            panic_with_error!(&env, MultisigError::ProposalAlreadyExecuted);
        }

        let proposal: Proposal = env.storage().instance().get(&DataKey::Proposal(proposal_id)).unwrap();
        
        if env.ledger().timestamp() > proposal.expires_at {
            panic_with_error!(&env, MultisigError::ProposalExpired);
        }

        let mut approvals: Vec<ProposalApproval> = env.storage().instance()
            .get(&DataKey::ProposalApprovals(proposal_id)).unwrap_or(Vec::new(&env));

        // Check if already approved
        for i in 0..approvals.len() {
            let approval = approvals.get_unchecked(i);
            if approval.signer == approver {
                panic_with_error!(&env, MultisigError::AlreadyApproved);
            }
        }

        let approval = ProposalApproval {
            signer: approver,
            approved_at: env.ledger().timestamp(),
        };

        approvals.push_back(approval);
        env.storage().instance().set(&DataKey::ProposalApprovals(proposal_id), &approvals);
    }

    pub fn revoke_approval(env: Env, proposal_id: u64, revoker: BytesN<32>) {
        Self::require_initialized(&env);
        
        if !env.storage().instance().has(&DataKey::Signer(revoker.clone())) {
            panic_with_error!(&env, MultisigError::UnknownSigner);
        }

        if !env.storage().instance().has(&DataKey::Proposal(proposal_id)) {
            panic_with_error!(&env, MultisigError::ProposalNotFound);
        }

        if env.storage().instance().has(&DataKey::ProposalExecuted(proposal_id)) {
            panic_with_error!(&env, MultisigError::ProposalAlreadyExecuted);
        }

        let mut approvals: Vec<ProposalApproval> = env.storage().instance()
            .get(&DataKey::ProposalApprovals(proposal_id)).unwrap_or(Vec::new(&env));

        let mut found = false;
        for i in 0..approvals.len() {
            let approval = approvals.get_unchecked(i);
            if approval.signer == revoker {
                approvals.remove(i);
                found = true;
                break;
            }
        }

        if !found {
            panic_with_error!(&env, MultisigError::SignerNotFound);
        }

        env.storage().instance().set(&DataKey::ProposalApprovals(proposal_id), &approvals);
    }

    pub fn execute_proposal(env: Env, proposal_id: u64) {
        Self::require_initialized(&env);
        
        // Check if proposal exists
        if !env.storage().instance().has(&DataKey::Proposal(proposal_id)) {
            panic_with_error!(&env, MultisigError::ProposalNotFound);
        }

        // Check if proposal is already executed
        if env.storage().instance().has(&DataKey::ProposalExecuted(proposal_id)) {
            panic_with_error!(&env, MultisigError::ProposalAlreadyExecuted);
        }

        let proposal: Proposal = env.storage().instance().get(&DataKey::Proposal(proposal_id)).unwrap();
        
        // Check if proposal is expired
        if env.ledger().timestamp() > proposal.expires_at {
            panic_with_error!(&env, MultisigError::ProposalExpired);
        }

        // Get approvals
        let approvals: Vec<ProposalApproval> = env.storage().instance()
            .get(&DataKey::ProposalApprovals(proposal_id)).unwrap_or(Vec::new(&env));

        let threshold: u32 = env.storage().instance().get(&DataKey::Threshold).unwrap();
        
        if approvals.len() < threshold {
            panic_with_error!(&env, MultisigError::InsufficientApprovals);
        }

        // Execute the token transfer first (external call)
        Self::execute_token_transfer(&env, &proposal);

        // Mark proposal as executed
        env.storage().instance().set(&DataKey::ProposalExecuted(proposal_id), &true);
        
        // Update proposal status
        let mut updated_proposal = proposal;
        updated_proposal.executed = true;
        env.storage().instance().set(&DataKey::Proposal(proposal_id), &updated_proposal);

        // Increment nonce
        let current_nonce: u64 = env.storage().instance().get(&DataKey::Nonce).unwrap();
        env.storage().instance().set(&DataKey::Nonce, &(current_nonce + 1));
    }

    pub fn get_proposal(env: Env, proposal_id: u64) -> Proposal {
        Self::require_initialized(&env);
        env.storage().instance().get(&DataKey::Proposal(proposal_id)).unwrap()
    }

    pub fn get_proposal_approvals(env: Env, proposal_id: u64) -> Vec<ProposalApproval> {
        Self::require_initialized(&env);
        env.storage().instance()
            .get(&DataKey::ProposalApprovals(proposal_id))
            .unwrap_or(Vec::new(&env))
    }

    pub fn is_proposal_executed(env: Env, proposal_id: u64) -> bool {
        Self::require_initialized(&env);
        env.storage().instance()
            .get(&DataKey::ProposalExecuted(proposal_id))
            .unwrap_or(false)
    }

    pub fn get_proposal_count(env: Env) -> u64 {
        Self::require_initialized(&env);
        env.storage().instance()
            .get(&DataKey::ProposalCount)
            .unwrap_or(0u64)
    }

    pub fn get_signer_change_proposal(env: Env, proposal_id: u64) -> SignerChangeProposal {
        Self::require_initialized(&env);
        env.storage().instance().get(&DataKey::SignerChangeProposal(proposal_id)).unwrap()
    }

    pub fn get_signer_change_approvals(env: Env, proposal_id: u64) -> Vec<SignerChangeApproval> {
        Self::require_initialized(&env);
        env.storage().instance()
            .get(&DataKey::SignerChangeApprovals(proposal_id))
            .unwrap_or(Vec::new(&env))
    }

    pub fn is_signer_change_executed(env: Env, proposal_id: u64) -> bool {
        Self::require_initialized(&env);
        env.storage().instance()
            .get(&DataKey::SignerChangeExecuted(proposal_id))
            .unwrap_or(false)
    }

    fn execute_token_transfer(env: &Env, proposal: &Proposal) {
        // Create a token client for the specified token
        let token_client = soroban_sdk::token::Client::new(env, &proposal.token_address);
        
        // Get the multisig contract address as the sender
        let multisig_address = env.current_contract_address();
        
        // Execute the transfer from multisig to recipient
        token_client.transfer(
            &multisig_address,
            &proposal.recipient,
            &proposal.amount,
        );
    }
}
