//! The unikernel image is produced at link time by `kraftld`, from inputs
//! cargo does not track: re-link when the variant's Kraftfile changes. A new
//! commit on the Unikraft branch is not seen: `touch Kraftfile` to re-link.

fn main() {
    println!("cargo:rerun-if-changed=Kraftfile");
}
