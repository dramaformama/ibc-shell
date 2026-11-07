//! Patch semantics helper: `Option<Option<T>>` where `None` = untouched (field absent),
//! `Some(None)` = set NULL (inherit), `Some(Some(v))` = set. TS: `field?: T | null`.

use serde::{Deserialize, Deserializer};

/// `deserialize_with` target for nullable patch fields. Use together with `#[serde(default)]`
/// so that an absent field stays `None` while an explicit `null` becomes `Some(None)`.
pub fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(de).map(Some)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct P {
        #[serde(default, deserialize_with = "super::double_option")]
        v: Option<Option<u16>>,
    }

    #[test]
    fn absent_null_value() {
        let absent: P = serde_json::from_str("{}").unwrap();
        let null: P = serde_json::from_str(r#"{"v":null}"#).unwrap();
        let set: P = serde_json::from_str(r#"{"v":22}"#).unwrap();
        assert_eq!(absent.v, None);
        assert_eq!(null.v, Some(None));
        assert_eq!(set.v, Some(Some(22)));
    }
}
