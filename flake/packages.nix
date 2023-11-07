{inputs, ...}: {
  perSystem = {
    system,
    pkgs,
    ...
  }: let
    craneLib =
      inputs.crane.lib.${system}.overrideToolchain
      inputs.fenix.packages.${system}.minimal.toolchain;
  in {
    packages.default = craneLib.buildPackage {
      src = craneLib.cleanCargoSource (craneLib.path ../.);
    };
  };
}
