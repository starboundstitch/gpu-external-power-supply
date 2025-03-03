{ pkgs ? import <nixpkgs> {} }:

with pkgs;

mkShell {
  buildInputs = [
    dfu-util
    gcc-arm-embedded
    probe-rs
    rust-analyzer
    rustup
    rustfmt
  ];
}
