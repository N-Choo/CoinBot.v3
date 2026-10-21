//! Compiles the analyzer proto into the engine gRPC client stubs — only when the `grpc` feature
//! is enabled, so non-grpc builds never need `protoc`.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("CARGO_FEATURE_GRPC").is_none() {
        return Ok(());
    }
    tonic_build::compile_protos("../../process/proto/analyzer.proto")?;
    Ok(())
}
