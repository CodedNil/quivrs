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
    vulkan-headers
  ];
  buildInputs = with pkgs; [
    openssl
  ];
  LD_LIBRARY_PATH =
    with pkgs;
    lib.makeLibraryPath [
      vulkan-loader
    ];
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
