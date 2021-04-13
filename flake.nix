{
  description = "Persway the friendly IPC daemon";

  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, fenix, nixpkgs, ... }:
    let
      package = pkgs: {
        pname = "persway";
        version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
        src = self;
        cargoSha256 = "sha256-c/30fqLOw1WvDRNgH+Su0i0kNzWPZ+qZJ6tHGS+UWjM=";
        doCheck = false;
        meta = {
          license = pkgs.stdenv.lib.licenses.mit;
          maintainers = [
            {
              email = "john@insane.se";
              github = "johnae";
              name = "John Axel Eriksson";
            }
          ];
        };
      };
      supportedSystems = [ "x86_64-linux" "x86_64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs supportedSystems (system: f system);
    in
      let
        pkgs = forAllSystems (system: import nixpkgs {
          localSystem = { inherit system; };
          overlays = [ fenix.overlay ];
        });
        rustPlatform = forAllSystems (system: pkgs.${system}.makeRustPlatform {
          inherit (fenix.packages.${system}.minimal) cargo rustc;
        });
      in
      {
        overlay = final: prev: {
          persway = prev.rustPlatform.buildRustPackage (package prev);
        };
        defaultPackage = forAllSystems (system: rustPlatform.${system}.buildRustPackage (package pkgs.${system}));
        devShell = forAllSystems (system: import ./devshell.nix { pkgs = pkgs.${system}; });
      };
}
