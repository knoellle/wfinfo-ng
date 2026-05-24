{}: let
  rust-overlay = (import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"));
  pkgs = (import <nixpkgs> {
    overlays = [ rust-overlay ];
  });
in
  pkgs.mkShell {
    nativeBuildInputs = with pkgs; [
      (rust-bin.stable.latest.default.override {
        extensions = [ "rust-src" ];
      })
      pkg-config
      cmake
      rustPlatform.bindgenHook
    ];

    buildInputs = with pkgs; [
      pkg-config
      cmake
      rustPlatform.bindgenHook
      openssl
      dbus
      fontconfig
      leptonica
      openssl
      # slop
      tesseract
      xorg.libX11
      xorg.libXcursor
      xorg.libXi
      xorg.libXrandr
      xorg.libXtst
    ];
  }
