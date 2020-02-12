let
  mozilla = import (builtins.fetchGit {
    url = "https://github.com/mozilla/nixpkgs-mozilla.git";
    ref = "master";
    rev = "36455d54de0b40d9432bba6d8207a5582210b3eb";
  });

  pkgs = import <nixpkgs> ({ overlays = [ mozilla ]; });

  rustChannel = pkgs.latest.rustChannels.stable;

  rust = rustChannel.rust.override {
    extensions = [ "rust-src" "clippy-preview" "rustfmt-preview" ];
  };

  cargo = rustChannel.cargo;

in with pkgs;
mkShell {
  buildInputs = [ rust cargo ];
  RUST_SRC_PATH = "${rustChannel.rust-src}/lib/rustlib/src/rust/src";
}
