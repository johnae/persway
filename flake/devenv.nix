{inputs, ...}: {
  imports = [
    inputs.devenv.flakeModule
  ];
  perSystem = {
    pkgs,
    lib,
    ...
  }: let
    inherit (builtins // lib) foldl' genList replaceStrings attrNames attrValues;
    esc = "\\e";
    ansiTable =
      {
        reset = "${esc}[0m";
        bold = "${esc}[1m";
        italic = "${esc}[3m";
        underline = "${esc}[4m";
      }
      // (foldl' (x: y: x // {"${toString y}" = "${esc}[38;5;${toString y}m";}) {} (genList (x: x) 256));

    ansiEscape =
      replaceStrings
      (map (key: "{${key}}") (attrNames ansiTable))
      (attrValues ansiTable);
  in {
    devenv.shells = lib.mapAttrs' (file: _: {
      name = builtins.replaceStrings [".nix"] [""] file;
      value = import "${../devenv}/${file}" {inherit pkgs inputs ansiEscape;};
    }) (builtins.readDir ../devenv);
  };
}
