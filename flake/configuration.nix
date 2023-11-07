{
  inputs,
  lib,
  self,
  ...
}: {
  imports = [
    ./devenv.nix
    ./packages.nix
  ];
  perSystem = {system, ...}: {
    _module.args.pkgs = import inputs.nixpkgs {
      inherit system;
      overlays = [
        inputs.fenix.overlays.default
      ];
      config = {};
    };
  };
}
