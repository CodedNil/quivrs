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
    pkg-config
    mold
    lld
    jq
    sqlite
  ];
  buildInputs = with pkgs; [
    openssl
  ];
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
