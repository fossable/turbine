{ pkgs ? import <nixpkgs> { } }:

with pkgs;

mkShell rec {
  nativeBuildInputs = [ pkg-config cargo rustc rust-analyzer rustfmt clippy ];
  buildInputs = [ openssl ];
  LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
}

