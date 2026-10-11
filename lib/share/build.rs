//! Build script: compiles the `wallet` and `analyzer` protobuf definitions into
//! Rust gRPC stubs.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../../process/proto/wallet.proto")?;
    tonic_build::compile_protos("../../process/proto/analyzer.proto")?;
    Ok(())
}
