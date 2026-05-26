{
  pkgs ? import <nixpkgs> { },
}:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustc
    cargo
    rustfmt
    clippy
    just
    dioxus-cli
    lld
    binaryen
    pkg-config
    surrealdb
  ];
  buildInputs = with pkgs; [
    openssl
  ];
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
