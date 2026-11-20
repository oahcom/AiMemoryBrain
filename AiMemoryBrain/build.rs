//! Build-time UCD binding hook.
//!
//! Step 1 only establishes the hook and tracked inputs. Step 2 must implement
//! strict lock validation before any validator binary is considered usable.

fn main() {
    println!("cargo:rerun-if-changed=ucd/15.1.0/ucd-lock.json");
    println!("cargo:rerun-if-changed=ucd/15.1.0/UnicodeData.txt");
    println!("cargo:rerun-if-changed=ucd/15.1.0/NormalizationTest.txt");
    println!("cargo:rerun-if-changed=ucd/15.1.0/DerivedNormalizationProps.txt");
    println!("cargo:rerun-if-changed=ucd/15.1.0/CompositionExclusions.txt");

    // Deliberately not a PASS: build-time fail-closed enforcement is a
    // Step 2 deliverable. Do not use this skeleton as a release validator.
}
