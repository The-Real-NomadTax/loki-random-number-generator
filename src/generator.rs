use loki_sdk::{Error, Result};
use serde::Serialize;
use std::collections::BTreeMap;

// Every supported integer is represented exactly by JavaScript and JSON clients.
pub const MAX_INTEGER: i64 = 9_007_199_254_740_991;
pub const MAX_COUNT: usize = 100;

#[derive(Clone, Debug, Serialize)]
pub struct Options {
    pub min: i64,
    pub max: i64,
    pub count: usize,
    pub unique: bool,
}

impl Options {
    pub fn parse(mut field: impl FnMut(&str) -> Option<String>) -> Result<Self> {
        fn integer(value: Option<String>, default: i64, label: &str) -> Result<i64> {
            match value {
                None => Ok(default),
                Some(value) => value
                    .trim()
                    .parse()
                    .map_err(|_| Error::bad(format!("{label} must be a whole number."))),
            }
        }
        let min = integer(field("min"), 1, "Minimum")?;
        let max = integer(field("max"), 100, "Maximum")?;
        let count = integer(field("count"), 1, "Quantity")?;
        let unique = match field("unique").as_deref() {
            None | Some("false" | "0") => false,
            Some("true" | "1") => true,
            _ => return Err(Error::bad("Unique must be true or false.")),
        };
        if !(-MAX_INTEGER..=MAX_INTEGER).contains(&min)
            || !(-MAX_INTEGER..=MAX_INTEGER).contains(&max)
        {
            return Err(Error::bad(format!(
                "Use integers between -{MAX_INTEGER} and {MAX_INTEGER}."
            )));
        }
        if min > max {
            return Err(Error::bad("Minimum must be less than or equal to maximum."));
        }
        if !(1..=MAX_COUNT as i64).contains(&count) {
            return Err(Error::bad(format!(
                "Quantity must be between 1 and {MAX_COUNT}."
            )));
        }
        if unique && count > max - min + 1 {
            return Err(Error::bad(
                "The range has too few numbers for that many unique results.",
            ));
        }
        Ok(Self {
            min,
            max,
            count: count as usize,
            unique,
        })
    }

    pub fn generate(&self) -> Result<Vec<i64>> {
        self.generate_with(|| {
            let mut bytes = [0; 8];
            getrandom::fill(&mut bytes)
                .map_err(|_| Error::new(503, "Randomness is unavailable. Please try again."))?;
            Ok(u64::from_le_bytes(bytes))
        })
    }

    fn generate_with(&self, mut random: impl FnMut() -> Result<u64>) -> Result<Vec<i64>> {
        let span = (self.max - self.min + 1) as u64;
        let mut swaps = BTreeMap::new();
        let mut numbers = Vec::with_capacity(self.count);
        for i in 0..self.count {
            let remaining = if self.unique { span - i as u64 } else { span };
            let index = below(remaining, &mut random)?;
            let offset = if self.unique {
                // Sparse Fisher-Yates: bounded memory and no duplicate retries,
                // even when drawing every integer from a small range.
                let selected = swaps.remove(&index).unwrap_or(index);
                let last = swaps.remove(&(remaining - 1)).unwrap_or(remaining - 1);
                if index != remaining - 1 {
                    swaps.insert(index, last);
                }
                selected
            } else {
                index
            };
            numbers.push(self.min + offset as i64);
        }
        Ok(numbers)
    }
}

fn below(bound: u64, random: &mut impl FnMut() -> Result<u64>) -> Result<u64> {
    // Reject the short remainder of the u64 domain to avoid modulo bias.
    let threshold = bound.wrapping_neg() % bound;
    loop {
        let value = random()?;
        if value >= threshold {
            return Ok(value % bound);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn options(fields: &[(&str, &str)]) -> Result<Options> {
        Options::parse(|name| {
            fields
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, v)| v.to_string())
        })
    }

    #[test]
    fn rejects_invalid_and_overflowing_inputs() {
        for fields in [
            vec![("min", "2"), ("max", "1")],
            vec![("min", "1.5")],
            vec![("min", "")],
            vec![("min", "-9007199254740992")],
            vec![("max", "9007199254740992")],
            vec![("count", "0")],
            vec![("count", "101")],
            vec![("count", "-1")],
            vec![("max", "999999999999999999999999")],
            vec![("unique", "yes")],
            vec![
                ("min", "1"),
                ("max", "2"),
                ("count", "3"),
                ("unique", "true"),
            ],
        ] {
            assert!(options(&fields).is_err(), "{fields:?}");
        }
    }

    #[test]
    fn equal_bounds_and_defaults_work() {
        assert_eq!(options(&[]).unwrap().count, 1);
        let opts = options(&[("min", "-7"), ("max", "-7"), ("count", "100")]).unwrap();
        assert_eq!(opts.generate_with(|| Ok(0)).unwrap(), vec![-7; 100]);
    }

    #[test]
    fn rejection_sampling_excludes_biased_tail() {
        let mut values = [0, 5, 6, u64::MAX].into_iter(); // threshold for ten is six.
        let mut random = || Ok(values.next().unwrap());
        assert_eq!(below(10, &mut random).unwrap(), 6);
        assert_eq!(below(10, &mut random).unwrap(), 5);
    }

    #[test]
    fn full_unique_draw_always_contains_the_range() {
        for seed in 1u64..=100 {
            let opts = options(&[
                ("min", "-49"),
                ("max", "50"),
                ("count", "100"),
                ("unique", "true"),
            ])
            .unwrap();
            let mut state = seed;
            let numbers = opts
                .generate_with(|| {
                    state = state.wrapping_mul(6364136223846793005u64).wrapping_add(1);
                    Ok(state)
                })
                .unwrap();
            assert_eq!(
                numbers.iter().copied().collect::<BTreeSet<_>>(),
                (-49..=50).collect()
            );
        }
    }

    #[test]
    fn supports_extreme_bounds_and_both_endpoints() {
        let opts = options(&[("min", "-9007199254740991"), ("max", "9007199254740991")]).unwrap();
        let span = (opts.max - opts.min + 1) as u64;
        assert_eq!(opts.generate_with(|| Ok(span)).unwrap(), vec![opts.min]);
        assert_eq!(opts.generate_with(|| Ok(span - 1)).unwrap(), vec![opts.max]);
    }

    #[test]
    fn entropy_failure_is_returned() {
        let error = options(&[])
            .unwrap()
            .generate_with(|| Err(Error::new(503, "unavailable")))
            .unwrap_err();
        assert_eq!(error.status(), 503);
    }
}
