let
  pkgs = import ./nix { };
  rustChannel = pkgs.latest.rustChannels.stable;

  rust = rustChannel.rust.override {
    extensions = [ "rust-src" "clippy-preview" "rustfmt-preview" ];
  };

  cargo = rustChannel.cargo;

in
pkgs.mkShell {
  buildInputs = with pkgs; [ niv rust cargo ];
  RUST_SRC_PATH = "${rustChannel.rust-src}/lib/rustlib/src/rust/src";
}
