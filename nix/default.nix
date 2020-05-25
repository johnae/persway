{ sources ? import ./sources.nix, ... }:
let
  nixpkgs =
    if builtins.hasAttr "nixpkgs" sources then
      sources.nixpkgs else <nixpkgs>;
in
import nixpkgs {
  overlays = [
    (_: _: { inherit sources; })
    (import sources.nixpkgs-mozilla)
  ];
}
