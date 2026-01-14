use anchor_lang::prelude::*;
use crate::constants::*;
use crate::error::ErrorCode;

pub fn calculate_time_multiplier(
    lock_duration: i64,
    numerator: u64,
    denominator: u64,
) -> Result<u64> {
    if lock_duration >= MAX_LOCK_DURATION {
        return Ok(numerator);
    }

    if lock_duration <= MIN_LOCK_DURATION {
        return Ok(denominator);
    }

    let duration_range = MAX_LOCK_DURATION - MIN_LOCK_DURATION;
    let duration_offset = lock_duration - MIN_LOCK_DURATION;
    let multiplier_range = numerator - denominator;

    let multiplier = denominator
        .checked_add(
            (duration_offset as u64)
                .checked_mul(multiplier_range)
                .ok_or(ErrorCode::MathOverflow)?
                .checked_div(duration_range as u64)
                .ok_or(ErrorCode::MathOverflow)?
        )
        .ok_or(ErrorCode::MathOverflow)?;

    Ok(multiplier)
}

pub fn calculate_current_ve_balance(
    initial_ve_amount: u64,
    lock_start_time: i64,
    unlock_time: i64,
    current_time: i64,
) -> Result<u64> {
    if current_time >= unlock_time {
        return Ok(0);
    }

    if current_time <= lock_start_time {
        return Ok(initial_ve_amount);
    }

    let total_duration = unlock_time
        .checked_sub(lock_start_time)
        .ok_or(ErrorCode::MathOverflow)?;

    let remaining_duration = unlock_time
        .checked_sub(current_time)
        .ok_or(ErrorCode::MathOverflow)?;

    let current_balance = (initial_ve_amount as u128)
        .checked_mul(remaining_duration as u128)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(total_duration as u128)
        .ok_or(ErrorCode::MathOverflow)? as u64;

    Ok(current_balance)
}
