let
  mozilla = import (builtins.fetchGit {
      url = "https://github.com/mozilla/nixpkgs-mozilla.git";
      ref = "master";
      rev = "9f35c4b09fd44a77227e79ff0c1b4b6a69dff533";
  });
  pkgs = import <nixpkgs> (
    { overlays = [ mozilla ]; }
  );
  rustChannel = pkgs.latest.rustChannels.stable;
in
  with pkgs; mkShell {
   buildInputs = [
     (rustChannel.rust.override { extensions = [ "clippy-preview" ]; })
   ];
   RUST_SRC_PATH = "${rustChannel.rust-src}/lib/rustlib/src/rust/src";
  }
