use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid amount: must be greater than 0")]
    InvalidAmount,

    #[msg("Invalid lock duration: must be between 7 days and 4 years")]
    InvalidLockDuration,

    #[msg("Existing lock has not expired yet")]
    ExistingLockNotExpired,

    #[msg("No existing lock found")]
    NoExistingLock,

    #[msg("Lock has expired, please withdraw first")]
    LockExpired,

    #[msg("Lock has not expired yet")]
    LockNotExpired,

    #[msg("No voting power available")]
    NoVotingPower,

    #[msg("No fees available to claim")]
    NoFeesToClaim,

    #[msg("Math overflow")]
    MathOverflow,
}
