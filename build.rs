fn main() {
    tonic_build::compile_protos("mcmultiplayer").unwrap_or_else(|e| println!("Failed to compile protobuffer: {:?}", e));
}
