{
  pkgs ? import <nixpkgs> { },
}:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustc
    cargo
    rustfmt
    clippy
    dioxus-cli
    lld
    binaryen
    pkg-config
    sqlx-cli
  ];
  buildInputs = with pkgs; [
    openssl
  ];
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
  DATABASE_URL = "sqlite://quivrs.db";
}
