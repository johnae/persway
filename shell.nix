{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  buildInputs = [
    (pkgs.rustChannel.rust.override { extensions = [ "rust-src" ]; })
  ];
}
