{
  description = "Persway the friendly IPC daemon";

  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.nix-misc = {
    url = "github:johnae/nix-misc";
    inputs.nixpkgs.follows = "nixpkgs";
  };

  inputs.mozilla = {
    url = "github:mozilla/nixpkgs-mozilla";
    flake = false;
  };

  outputs = { self, nixpkgs, flake-utils, mozilla, ... }@inputs:
    {
      overlay = final: prev: {
        persway = final.callPackage ./. { inherit self; };
      };
    } // (
      flake-utils.lib.eachDefaultSystem
        (system:
          let
            rustOverlay = final: prev:
              let
                rustChannel = prev.rustChannelOf {
                  channel = "1.49.0";
                  sha256 = "sha256-KCh2UBGtdlBJ/4UOqZlxUtcyefv7MH1neoVNV4z0nWs=";
                };
              in
              {
                inherit rustChannel;
                rustc = rustChannel.rust;
                cargo = rustChannel.rust;
              };


            pkgs = import nixpkgs {
              inherit system;
              overlays = [
                self.overlay
                (import "${mozilla}/rust-overlay.nix")
                rustOverlay
              ];
            };
          in
          {
            defaultPackage =
              pkgs.persway;
            packages = flake-utils.lib.flattenTree {
              persway = self.defaultPackage;
            };
            apps.persway = flake-utils.lib.mkApp {
              drv = self.persway;
              exePath = "/bin/persway";
            };
            defaultApp = self.apps.persway;
            devShell = import ./shell.nix { inherit pkgs; };
          }
        )
    );
}
