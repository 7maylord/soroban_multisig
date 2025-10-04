use soroban_sdk::{contracttype, Address, BytesN, String};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Initialized,
    SignerCount,
    Signer(BytesN<32>),
    Threshold,
    Nonce,
    ProposalCount,
    Proposal(u64),
    ProposalApprovals(u64),
    ProposalExecuted(u64),
    SignerChangeProposal(u64),
    SignerChangeApprovals(u64),
    SignerChangeExecuted(u64),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub id: u64,
    pub proposer: BytesN<32>,
    pub token_address: Address,
    pub recipient: Address,
    pub amount: i128,
    pub reason: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub executed: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalApproval {
    pub signer: BytesN<32>,
    pub approved_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignerChangeProposal {
    pub id: u64,
    pub proposer: BytesN<32>,
    pub change_type: String, // "add" or "remove"
    pub signer: BytesN<32>,
    pub created_at: u64,
    pub expires_at: u64,
    pub executed: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignerChangeApproval {
    pub signer: BytesN<32>,
    pub approved_at: u64,
}
