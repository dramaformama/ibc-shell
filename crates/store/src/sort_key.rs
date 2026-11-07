//! Fractional index keys (base-62), in the spirit of the well-known "fractional-indexing"
//! scheme: an integer part whose length is encoded by the head letter, followed by an optional
//! fractional part. Keys compare correctly as plain strings, appending or prepending grows a key
//! only by one char per 62^n steps, and an insert between neighbours never renumbers anything.

const DIGITS: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const ZERO: u8 = b'0';

fn smallest_int() -> String {
    format!("A{}", "0".repeat(26))
}

fn digit_val(c: u8) -> Option<usize> {
    DIGITS.iter().position(|&d| d == c)
}

fn int_len(head: u8) -> Option<usize> {
    match head {
        b'a'..=b'z' => Some(usize::from(head - b'a') + 2),
        b'A'..=b'Z' => Some(usize::from(b'Z' - head) + 2),
        _ => None,
    }
}

fn split(key: &str) -> Option<(&str, &str)> {
    let n = int_len(*key.as_bytes().first()?)?;
    if n > key.len() || !key.is_char_boundary(n) {
        return None;
    }
    Some(key.split_at(n))
}

fn valid(key: &str) -> bool {
    key != smallest_int()
        && key.bytes().all(|c| digit_val(c).is_some())
        && split(key).is_some_and(|(_, f)| !f.ends_with('0'))
}

/// Midpoint of two fractional parts (`a` < `b`, `a` has no trailing zero); `b = None` = 1.0.
fn midpoint(a: &str, b: Option<&str>) -> Option<String> {
    if let Some(b) = b {
        let (ab, bb) = (a.as_bytes(), b.as_bytes());
        let mut n = 0;
        while n < bb.len() && ab.get(n).copied().unwrap_or(ZERO) == bb[n] {
            n += 1;
        }
        if n > 0 {
            let rest = midpoint(a.get(n..).unwrap_or(""), Some(&b[n..]))?;
            return Some(format!("{}{rest}", &b[..n]));
        }
    }
    let da = a.bytes().next().map_or(Some(0), digit_val)?;
    let db = match b {
        Some(b) => digit_val(*b.as_bytes().first()?)?,
        None => 62,
    };
    if db > da + 1 {
        return Some(char::from(DIGITS[(da + db).div_ceil(2)]).to_string());
    }
    if let Some(b) = b.filter(|b| b.len() > 1) {
        return Some(b[..1].to_string());
    }
    let tail = midpoint(a.get(1..).unwrap_or(""), None)?;
    Some(format!("{}{tail}", char::from(DIGITS[da])))
}

fn increment(int: &str) -> Option<String> {
    let mut b = int.as_bytes().to_vec();
    let head = b[0];
    let mut carry = true;
    for c in b[1..].iter_mut().rev() {
        let d = digit_val(*c)? + 1;
        if d == 62 {
            *c = ZERO;
        } else {
            *c = DIGITS[d];
            carry = false;
            break;
        }
    }
    if carry {
        match head {
            b'Z' => return Some("a0".into()),
            b'z' => return None,
            _ => {}
        }
        b[0] = head + 1;
        if b[0] > b'a' {
            b.push(ZERO);
        } else {
            b.pop();
        }
    }
    String::from_utf8(b).ok()
}

fn decrement(int: &str) -> Option<String> {
    let mut b = int.as_bytes().to_vec();
    let head = b[0];
    let mut borrow = true;
    for c in b[1..].iter_mut().rev() {
        let d = digit_val(*c)?;
        if d == 0 {
            *c = DIGITS[61];
        } else {
            *c = DIGITS[d - 1];
            borrow = false;
            break;
        }
    }
    if borrow {
        match head {
            b'a' => return Some("Zz".into()),
            b'A' => return None,
            _ => {}
        }
        b[0] = head - 1;
        if b[0] < b'Z' {
            b.push(DIGITS[61]);
        } else {
            b.pop();
        }
    }
    String::from_utf8(b).ok()
}

/// Returns a key strictly between `a` and `b` (`None` = open end). A `None` result means an
/// invalid input key, `a >= b`, or key space exhausted.
#[must_use]
pub fn between(a: Option<&str>, b: Option<&str>) -> Option<String> {
    if a.is_some_and(|k| !valid(k)) || b.is_some_and(|k| !valid(k)) {
        return None;
    }
    match (a, b) {
        (Some(a), Some(b)) if a >= b => None,
        (None, None) => Some("a0".into()),
        (None, Some(b)) => {
            let (ib, fb) = split(b)?;
            if ib == smallest_int() {
                return Some(format!("{ib}{}", midpoint("", Some(fb))?));
            }
            if ib < b {
                return Some(ib.to_string());
            }
            decrement(ib)
        }
        (Some(a), None) => {
            let (ia, fa) = split(a)?;
            match increment(ia) {
                Some(i) => Some(i),
                None => Some(format!("{ia}{}", midpoint(fa, None)?)),
            }
        }
        (Some(a), Some(b)) => {
            let (ia, fa) = split(a)?;
            let (ib, fb) = split(b)?;
            if ia == ib {
                return Some(format!("{ia}{}", midpoint(fa, Some(fb))?));
            }
            match increment(ia) {
                Some(i) if i.as_str() < b => Some(i),
                _ => Some(format!("{ia}{}", midpoint(fa, None)?)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::between;

    #[test]
    fn first_and_ordering() {
        assert_eq!(between(None, None).as_deref(), Some("a0"));
        let a = between(None, None).unwrap();
        let b = between(Some(&a), None).unwrap();
        assert!(a < b);
        let m = between(Some(&a), Some(&b)).unwrap();
        assert!(a < m && m < b);
        let z = between(None, Some(&a)).unwrap();
        assert!(z < a);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(between(Some("a1"), Some("a0")).is_none());
        assert!(between(Some("a1"), Some("a1")).is_none());
        assert!(between(Some("!"), None).is_none());
        assert!(between(Some("a00"), None).is_none()); // trailing zero in fraction
    }

    #[test]
    fn ten_thousand_appends_stay_short() {
        let mut last: Option<String> = None;
        for _ in 0..10_000 {
            let k = between(last.as_deref(), None).unwrap();
            if let Some(l) = &last {
                assert!(l < &k);
            }
            assert!(k.len() < 40, "len {}", k.len());
            last = Some(k);
        }
    }

    #[test]
    fn ten_thousand_prepends_stay_short() {
        let mut first: Option<String> = None;
        for _ in 0..10_000 {
            let k = between(None, first.as_deref()).unwrap();
            if let Some(f) = &first {
                assert!(&k < f);
            }
            assert!(k.len() < 40, "len {}", k.len());
            first = Some(k);
        }
    }

    #[test]
    fn repeated_middle_inserts_remain_ordered() {
        let lo = "a0".to_string();
        let mut hi = between(Some(&lo), None).unwrap();
        for _ in 0..500 {
            let m = between(Some(&lo), Some(&hi)).unwrap();
            assert!(lo < m && m < hi);
            hi = m;
        }
    }

    #[test]
    fn random_inserts_keep_total_order() {
        let mut keys = vec![between(None, None).unwrap()];
        let mut seed = 12345u64;
        for _ in 0..2000 {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let i = (seed >> 33) as usize % (keys.len() + 1);
            let a = if i == 0 {
                None
            } else {
                Some(keys[i - 1].as_str())
            };
            let b = keys.get(i).map(String::as_str);
            let k = between(a, b).unwrap();
            keys.insert(i, k);
        }
        assert!(keys.windows(2).all(|w| w[0] < w[1]));
    }
}
