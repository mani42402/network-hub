// tonic 0.14 moved prost-based codegen out of `tonic-build` into
// `tonic-prost-build`. Worth recording: the daemon's real build (WP-M0.2) needs
// the same pair, and the split is easy to miss when following older examples.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .build_client(false)
        .compile_protos(&["../proto/spike.proto"], &["../proto"])?;
    println!("cargo:rerun-if-changed=../proto/spike.proto");
    Ok(())
}
