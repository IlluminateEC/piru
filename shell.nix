{
  pkgs ? import <nixpkgs> { },
}:
let
  libraries = with pkgs; [
    libx11
    libxcursor
    libxrandr
    libxi
    libxcb
    libxkbcommon
    vulkan-loader
    wayland
  ];

  packages =
    libraries
    ++ (with pkgs; [
      directx-shader-compiler
      pkg-config
    ]);
in
pkgs.mkShell {
  packages = packages;

  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath libraries;
}
