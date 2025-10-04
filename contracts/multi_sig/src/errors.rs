use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MultisigError {
    NotInitialized = 0,
    AlreadyInitialized = 1,
    InvalidThreshold = 3,
    EmptySignersList = 4,
    DuplicateSigner = 5,
    SignerNotFound = 6,
    ThresholdExceedsSigners = 7,
    InvalidNonce = 8,
    UnknownSigner = 9,
    ProposalNotFound = 13,
    ProposalAlreadyExecuted = 14,
    ProposalExpired = 15,
    AlreadyApproved = 16,
    InsufficientApprovals = 17,
    InvalidProposal = 18,
    SignerChangeNotFound = 19,
    SignerChangeAlreadyExecuted = 20,
    SignerChangeExpired = 21,
    SignerChangeAlreadyApproved = 22,
    InsufficientSignerChangeApprovals = 23,
    InvalidExpiryTime = 24,
}
