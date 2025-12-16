fn main() {
    tonic_prost_build::configure()
        .out_dir("src/generated")
        .compile_protos(&["proto/hello.proto", "proto/lrc_user_rpc.proto"], &["proto"])
        .unwrap();
}
