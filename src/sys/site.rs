//! The identity of one site of a gate, which the ratchet, the gap report and the `secrets`
//! fingerprint all hash the same way. Spec 11.2.

/// The identity of one site of a gate, from its file and its text. FNV-1a, written out here, so
/// one site keeps one id across builds of klin.
pub fn id(gate: &str, file: &str, text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let parts = [gate.as_bytes(), file.as_bytes(), text.as_bytes()];
    for byte in parts.join(&0u8) {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
    }
    format!("{hash:016x}")
}
