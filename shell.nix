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
    sqlx-cli
    sqlite
  ];
  buildInputs = with pkgs; [
    openssl
  ];
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
  DATABASE_URL = "sqlite://quivrs.db";
}
