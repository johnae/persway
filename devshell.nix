{ pkgs }:

pkgs.mkShell {
  buildInputs = [
    (pkgs.rust-nightly.latest.withComponents [
      "cargo"
      "clippy-preview"
      "rust-src"
      "rust-std"
      "rustc"
      "rustfmt-preview"
    ])
    pkgs.rust-analyzer-nightly
    pkgs.gcc
    pkgs.openssl
    pkgs.pkg-config
    pkgs.skim
  ];
}
