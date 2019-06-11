let
  mozilla = import (builtins.fetchGit {
      url = "https://github.com/mozilla/nixpkgs-mozilla.git";
      ref = "master";
      rev = "9f35c4b09fd44a77227e79ff0c1b4b6a69dff533";
  });
in
  with import <nixpkgs> (
    { overlays = [ mozilla ]; }
  );

  mkShell {
   buildInputs = [ latest.rustChannels.stable.rust rustracer ];
   RUST_SRC_PATH = "${latest.rustChannels.stable.rust-src}/lib/rustlib/src/rust/src";
  }
