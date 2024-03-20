{ pkgs, lib, stdenv, rust }:
let
  manifest = (pkgs.lib.importTOML ../healthpi-loader/Cargo.toml).package;
  rustPlatform = pkgs.makeRustPlatform {
    rustc = rust;
    cargo = rust;
  };
in rustPlatform.buildRustPackage rec {
  pname = manifest.name;
  version = manifest.version;

  src = ../.;
  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [
    pkgs.pkg-config
  ];

  buildInputs = [ 
    pkgs.dbus
  ] ++ lib.optionals stdenv.isDarwin [
    pkgs.darwin.apple_sdk.frameworks.AppKit
    pkgs.darwin.apple_sdk.frameworks.CoreBluetooth
  ];
}
