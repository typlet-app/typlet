// SPDX-License-Identifier: GPL-3.0-or-later
//! The public key in `tauri.conf.json` must belong to the release signing
//! key: otherwise installed copies reject every update (ADR 0015).
//!
//! `fixtures/updater-key-check.txt` was signed once with the release key
//! (`tauri signer sign --app-version 0.0.0`); this test verifies it the way
//! the updater verifies installers.
#![allow(clippy::unwrap_used)] // test code: a panic is the failure report

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use minisign_verify::{PublicKey, Signature};

fn decode(text: &str) -> String {
    String::from_utf8(STANDARD.decode(text.trim()).unwrap()).unwrap()
}

#[test]
fn configured_public_key_verifies_the_release_signature() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let updater = &config["plugins"]["updater"];
    let public_key = PublicKey::decode(&decode(updater["pubkey"].as_str().unwrap())).unwrap();
    let signature =
        Signature::decode(&decode(include_str!("fixtures/updater-key-check.txt.sig"))).unwrap();
    let data = include_bytes!("fixtures/updater-key-check.txt");

    public_key.verify(data, &signature, true).unwrap();
    // Release signatures carry the version, and the updater insists on it.
    assert!(signature.trusted_comment().contains("version:0.0.0"));
    assert_eq!(updater["requireSignedVersion"], true);

    let mut tampered = data.to_vec();
    tampered[0] ^= 1;
    assert!(public_key.verify(&tampered, &signature, true).is_err());
}

#[test]
fn updates_come_from_the_latest_github_release() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    assert_eq!(
        config["plugins"]["updater"]["endpoints"],
        serde_json::json!([
            "https://github.com/typlet-app/typlet/releases/latest/download/latest.json"
        ])
    );
}
