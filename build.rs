fn main() {
    prost_build::compile_protos(
        &["actor-delegate.proto", "actor-pinner.proto"],
        &["../tea-codec/proto"],
    )
    .unwrap();
}
