use std::{env, path::PathBuf};

use prost_wkt_build::{FileDescriptorSet, Message};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let descriptor_file = out.join("descriptors.bin");
    let mut config = prost_build::Config::new();
    // Older protoc versions need this flag for optional types
    config.protoc_arg("--experimental_allow_proto3_optional");
    config
        .type_attribute(".", "#[derive(serde::Serialize,serde::Deserialize)]")
        .extern_path(".google.protobuf.Any", "::prost_wkt_types::Any")
        .extern_path(".google.protobuf.Timestamp", "::prost_wkt_types::Timestamp")
        .extern_path(".google.protobuf.Value", "::prost_wkt_types::Value")
        .file_descriptor_set_path(&descriptor_file);
    tonic_build::configure().compile_with_config(
        config,
        &["protos/messages.proto", "protos/backend.proto", "protos/ivy_daemon.proto"],
        &["protos"],
    )?;
    let descriptor_bytes = std::fs::read(descriptor_file).unwrap();
    let descriptor = FileDescriptorSet::decode(&descriptor_bytes[..]).unwrap();
    prost_wkt_build::add_serde(out, descriptor);
    Ok(())
}
