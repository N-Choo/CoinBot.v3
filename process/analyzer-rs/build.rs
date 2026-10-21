fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("CARGO_FEATURE_GRPC").is_none() {
        return Ok(());
    }
    tonic_build::compile_protos("../../process/proto/analyzer.proto")?;
    Ok(())
}
