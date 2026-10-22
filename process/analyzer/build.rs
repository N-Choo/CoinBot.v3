fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../../process/proto/analyzer.proto");
    if std::env::var_os("CARGO_FEATURE_GRPC").is_none() {
        return Ok(());
    }
    tonic_build::compile_protos("../../process/proto/analyzer.proto")?;
    Ok(())
}
