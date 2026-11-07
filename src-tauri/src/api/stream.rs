//! Raw-bytes streaming (ADR-004): `Channel<Vec<u8>>` would be a JSON number array, so the
//! terminal stream uses [`RawBytes`], delivered to JS as an `ArrayBuffer`.

use tauri::ipc::{InvokeResponseBody, IpcResponse};

/// Raw byte chunk sent over a `Channel<RawBytes>`. TS type: `ArrayBuffer`.
/// Intentionally NOT `Serialize`, so it can never fall back to JSON.
#[derive(Clone, PartialEq, Eq)]
pub struct RawBytes(pub Vec<u8>);

impl std::fmt::Debug for RawBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RawBytes({} bytes)", self.0.len())
    }
}

impl IpcResponse for RawBytes {
    fn body(self) -> tauri::Result<InvokeResponseBody> {
        Ok(InvokeResponseBody::Raw(self.0))
    }
}

impl specta::Type for RawBytes {
    fn definition(_: &mut specta::Types) -> specta::datatype::DataType {
        specta::datatype::DataType::Reference(specta_typescript::define("ArrayBuffer"))
    }
}
