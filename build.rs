use ttrpc_codegen::{Codegen, Customize, ProtobufCustomize};

fn main() -> std::io::Result<()> {
    let protos = &["protos/attestation_agent.proto"];
    let includes = &["protos"];

    // Configure protobuf code generation
    let protobuf_customized = ProtobufCustomize::default()
        .gen_mod_rs(false);

    Codegen::new()
        .out_dir("src/rpc_generated")
        .inputs(protos)
        .includes(includes)
        .rust_protobuf()
        .customize(Customize {
            async_all: true,
            ..Default::default()
        })
        .rust_protobuf_customize(protobuf_customized)
        .run()
        .expect("Failed to run ttrpc codegen");

    Ok(())
} 