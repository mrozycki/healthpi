{ pkgs, rust }: 
pkgs.mkShell {
  inputsFrom = [ (pkgs.callPackage ./default.nix { inherit rust; }) ];

  buildInputs = with pkgs; [
    sqlx-cli
  ];
}
