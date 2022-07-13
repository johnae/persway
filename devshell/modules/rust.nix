{
  lib,
  config,
  pkgs,
  ...
}: let
  l = lib // builtins;
  cfg = config.language.rust;
in
  with l; {
    options.language.rust = {
      packageSet = mkOption {
        type = types.either types.attrs types.str;
        default = pkgs.rustPackages;
        defaultText = "pkgs.rustPlatform";
        description = "Which rust package set to use";
        apply = v:
          if isString v
          then attrByPath (splitString "." v) null pkgs
          else v;
      };
      tools = mkOption {
        type = types.listOf types.str;
        default = [
          "rustc"
          "cargo"
          "clippy"
          "rustfmt"
        ];
        description = "Which rust tools to pull from the platform package set";
      };
    };

    config = {
      devshell.packages = map (tool: cfg.packageSet.${tool}) cfg.tools;
      env =
        (
          if elem "rust-src" cfg.tools
          then [
            {
              name = "RUST_SRC_PATH";
              eval = "$DEVSHELL_DIR/lib/rustlib/src/rust/library";
            }
          ]
          else []
        )
        ++ [
          {
            name = "PKG_CONFIG_PATH";
            eval = "$DEVSHELL_DIR/lib/pkgconfig";
          }
          {
            # On darwin for example enables finding of libiconv
            name = "LIBRARY_PATH";
            # append in case it needs to be modified
            eval = "$DEVSHELL_DIR/lib";
          }
          {
            # some *-sys crates require additional includes
            name = "CFLAGS";
            # append in case it needs to be modified
            eval = "\"-I $DEVSHELL_DIR/include ${lib.optionalString pkgs.stdenv.isDarwin "-iframework $DEVSHELL_DIR/Library/Frameworks"}\"";
          }
        ]
        ++ lib.optionals pkgs.stdenv.isDarwin [
          {
            # On darwin for example required for some *-sys crate compilation
            name = "RUSTFLAGS";
            # append in case it needs to be modified
            eval = "\"-L framework=$DEVSHELL_DIR/Library/Frameworks\"";
          }
          {
            # rustdoc uses a different set of flags
            name = "RUSTDOCFLAGS";
            # append in case it needs to be modified
            eval = "\"-L framework=$DEVSHELL_DIR/Library/Frameworks\"";
          }
        ];
    };
  }
