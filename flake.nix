{
  description = "Persway";

  inputs = {
    crane.url = "github:ipetkov/crane";
    devenv.url = "github:cachix/devenv";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
    fenix.url = "github:nix-community/fenix";
    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-utils.url = "github:numtide/flake-utils";
    mk-shell-bin.url = "github:rrbutani/nix-mk-shell-bin";
    nix2container.inputs.flake-utils.follows = "flake-utils";
    nix2container.inputs.nixpkgs.follows = "nixpkgs";
    nix2container.url = "github:nlewo/nix2container";
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux" "x86_64-darwin" "aarch64-linux" "aaarch64-darwin"];
      imports = [
        ./flake/configuration.nix
      ];
    };
}
