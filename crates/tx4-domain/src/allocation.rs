//! Deterministic weighted money allocation (ADR-006 §11).
//!
//! Uses floor shares + largest-remainder distribution. Half-even (§9) is not
//! used for this primitive.

use crate::error::DomainError;
use crate::money::Money;

/// Allocate `source` across `weights` per ADR-006 §11.
///
/// Conservation: `sum(parts) == source.amount`. Negative sources are rejected.
pub fn allocate(source: &Money, weights: &[u64]) -> Result<Vec<Money>, DomainError> {
    if weights.is_empty() {
        return Err(DomainError::InvalidAllocation);
    }
    if source.amount() < 0 {
        return Err(DomainError::InvalidAllocation);
    }

    let w_sum: u128 = weights.iter().map(|&w| u128::from(w)).sum();
    if w_sum == 0 {
        return Err(DomainError::InvalidAllocation);
    }

    let s = source.amount() as u128;
    let currency = source.currency().clone();
    let n = weights.len();

    // Zero source → all zero parts.
    if s == 0 {
        return Ok((0..n).map(|_| Money::new(0, currency.clone())).collect());
    }

    let mut base: Vec<u128> = Vec::with_capacity(n);
    let mut rem: Vec<u128> = Vec::with_capacity(n);
    let mut t: u128 = 0;

    for &w in weights {
        let num = s
            .checked_mul(u128::from(w))
            .ok_or(DomainError::MoneyOverflow)?;
        let b = num / w_sum;
        let r = num % w_sum;
        t = t.checked_add(b).ok_or(DomainError::MoneyOverflow)?;
        base.push(b);
        rem.push(r);
    }

    let r_gap = s.checked_sub(t).ok_or(DomainError::MoneyOverflow)?;
    // R < n always for this algorithm when inputs are valid.
    if r_gap >= n as u128 {
        return Err(DomainError::InvalidAllocation);
    }

    let mut parts_u = base;
    if r_gap > 0 {
        // Eligible indices: w[i] > 0. Sort by rem desc, then index asc.
        let mut eligible: Vec<usize> = weights
            .iter()
            .enumerate()
            .filter(|(_, w)| **w > 0)
            .map(|(i, _)| i)
            .collect();
        eligible.sort_by(|&a, &b| rem[b].cmp(&rem[a]).then_with(|| a.cmp(&b)));
        let take = usize::try_from(r_gap).map_err(|_| DomainError::MoneyOverflow)?;
        if take > eligible.len() {
            return Err(DomainError::InvalidAllocation);
        }
        for &i in eligible.iter().take(take) {
            parts_u[i] = parts_u[i]
                .checked_add(1)
                .ok_or(DomainError::MoneyOverflow)?;
        }
    }

    let mut out = Vec::with_capacity(n);
    let mut check: u128 = 0;
    for amount_u in parts_u {
        check = check
            .checked_add(amount_u)
            .ok_or(DomainError::MoneyOverflow)?;
        let amount = i64::try_from(amount_u).map_err(|_| DomainError::MoneyOverflow)?;
        out.push(Money::new(amount, currency.clone()));
    }
    if check != s {
        return Err(DomainError::InvalidAllocation);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CurrencyId;

    fn idr(a: i64) -> Money {
        Money::new(a, CurrencyId::new("IDR").unwrap())
    }

    #[test]
    fn equal_split_no_remainder() {
        let parts = allocate(&idr(100), &[1, 1]).unwrap();
        assert_eq!(parts, vec![idr(50), idr(50)]);
    }

    #[test]
    fn adr_example_100_over_1_1_1() {
        let parts = allocate(&idr(100), &[1, 1, 1]).unwrap();
        assert_eq!(parts, vec![idr(34), idr(33), idr(33)]);
    }

    #[test]
    fn adr_example_100_over_1_2_3() {
        let parts = allocate(&idr(100), &[1, 2, 3]).unwrap();
        assert_eq!(parts, vec![idr(17), idr(33), idr(50)]);
    }

    #[test]
    fn weighted_2_1() {
        // W=3, num=[200,100], base=[66,33], rem=[2,1], T=99, R=1 → +1 to idx 0
        assert_eq!(
            allocate(&idr(100), &[2, 1]).unwrap(),
            vec![idr(67), idr(33)]
        );
    }

    #[test]
    fn rejects_empty_zero_weights_and_negative_source() {
        assert!(allocate(&idr(10), &[]).is_err());
        assert!(allocate(&idr(10), &[0, 0]).is_err());
        assert!(allocate(&idr(-1), &[1]).is_err());
    }

    #[test]
    fn zero_total() {
        let parts = allocate(&idr(0), &[1, 2, 3]).unwrap();
        assert_eq!(parts, vec![idr(0), idr(0), idr(0)]);
    }

    #[test]
    fn zero_weight_recipient_gets_nothing() {
        let parts = allocate(&idr(10), &[0, 1, 1]).unwrap();
        assert_eq!(parts[0].amount(), 0);
        assert_eq!(parts.iter().map(|m| m.amount()).sum::<i64>(), 10);
    }
}
