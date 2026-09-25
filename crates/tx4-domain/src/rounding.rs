//! Half-even rounding onto an integer atomic grid (ADR-006 §9).
//!
//! Operates on exact rationals `numerator / denominator` using integer arithmetic only.

use crate::DomainError;

/// Round the exact rational `numerator / denominator` to the nearest integer using
/// half-even (banker's) rounding, then require the result to fit in `i64`.
///
/// `denominator` must be positive.
pub fn round_half_even_rational(numerator: i128, denominator: i128) -> Result<i64, DomainError> {
    if denominator <= 0 {
        return Err(DomainError::InvalidRounding);
    }

    // Sign of the rational; zero numerator → zero (no negative zero).
    if numerator == 0 {
        return Ok(0);
    }

    let s: i128 = if numerator >= 0 { 1 } else { -1 };
    let abs_n = numerator.unsigned_abs();
    let den = denominator as u128;

    let i = abs_n / den;
    let rem = abs_n % den;

    // Compare rem/den to 1/2 without floats: rem*2 ? den
    let twice_rem = rem.checked_mul(2).ok_or(DomainError::InvalidRounding)?;
    let u = if twice_rem < den {
        i
    } else if twice_rem > den {
        i.checked_add(1).ok_or(DomainError::InvalidRounding)?
    } else {
        // Exact half: if i even stay, if odd go to i+1
        if i.is_multiple_of(2) {
            i
        } else {
            i.checked_add(1).ok_or(DomainError::InvalidRounding)?
        }
    };

    let signed = s
        .checked_mul(i128::try_from(u).map_err(|_| DomainError::InvalidRounding)?)
        .ok_or(DomainError::InvalidRounding)?;
    i64::try_from(signed).map_err(|_| DomainError::MoneyOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_examples() {
        // x = n/d
        assert_eq!(round_half_even_rational(20, 10).unwrap(), 2); // 2.0
        assert_eq!(round_half_even_rational(24, 10).unwrap(), 2); // 2.4
        assert_eq!(round_half_even_rational(26, 10).unwrap(), 3); // 2.6
        assert_eq!(round_half_even_rational(25, 10).unwrap(), 2); // 2.5 even
        assert_eq!(round_half_even_rational(35, 10).unwrap(), 4); // 3.5 odd → 4
        assert_eq!(round_half_even_rational(-24, 10).unwrap(), -2);
        assert_eq!(round_half_even_rational(-26, 10).unwrap(), -3);
        assert_eq!(round_half_even_rational(-25, 10).unwrap(), -2);
        assert_eq!(round_half_even_rational(-35, 10).unwrap(), -4);
        assert_eq!(round_half_even_rational(5, 10).unwrap(), 0); // 0.5
        assert_eq!(round_half_even_rational(-5, 10).unwrap(), 0); // -0.5 → 0
    }

    #[test]
    fn rejects_non_positive_denominator() {
        assert!(round_half_even_rational(1, 0).is_err());
        assert!(round_half_even_rational(1, -1).is_err());
    }
}
