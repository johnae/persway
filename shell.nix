with import <nixpkgs> {};

mkShell {
 buildInputs = [ latest.rustChannels.stable.rust rustracer ];
 RUST_SRC_PATH = "${latest.rustChannels.stable.rust-src}/lib/rustlib/src/rust/src";
}
