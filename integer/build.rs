//! Declare the `force_bits` cfg that the test workflow sets via
//! `RUSTFLAGS=--cfg force_bits=...` (16/32/64) so the matrix can exercise
//! the alternative `Word` widths. Without this declaration, stable Rust's
//! `unexpected_cfgs` lint (added by default in 1.80) fires on the three
//! `#[cfg(force_bits = "...")]` sites in `src/arch/mod.rs`.
fn main() {
    // Single-colon `cargo:` form for portability. The directive itself
    // was stabilised in Rust 1.80; older cargo (down to our MSRV) just
    // silently ignores unknown `cargo:` directives, which is fine because
    // the `unexpected_cfgs` lint they'd suppress doesn't exist at older
    // versions either.
    println!(r#"cargo:rustc-check-cfg=cfg(force_bits, values("16", "32", "64"))"#);
}
