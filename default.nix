{ pkgs ? import <nixpkgs> { }
, lib ? import <nixpkgs/lib>
, nixos-appstream-data ? (import ./nixos-appstream-data/default.nix { set = "all"; stdenv = pkgs.stdenv; lib = pkgs.lib; pkgs = pkgs; })
}: pkgs.stdenv.mkDerivation rec {
  pname = "nix-software-center";
  version = "0.1.2";

  src = [ ./. ];

  cargoDeps = pkgs.rustPlatform.importCargoLock {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = with pkgs; [
    appstream-glib
    polkit
    gettext
    desktop-file-utils
    meson
    ninja
    pkg-config
    git
    wrapGAppsHook4
  ] ++ (with pkgs.rustPlatform; [
    cargoSetupHook
    cargo
    rustc
  ]);

  buildInputs = with pkgs; [
    gdk-pixbuf
    glib
    gtk4
    gtksourceview5
    libadwaita
    libxml2
    openssl
    wayland
    adwaita-icon-theme
    desktop-file-utils
    nixos-appstream-data
  ];

  patchPhase = ''
    substituteInPlace ./src/lib.rs \
        --replace "/usr/share/app-info" "${nixos-appstream-data}/share/app-info"
  '';

  postInstall = ''
    wrapProgram $out/bin/nix-software-center --prefix PATH : '${lib.makeBinPath [
      pkgs.gnome-console
      pkgs.gtk3 # provides gtk-launch
      pkgs.sqlite
    ]}'
  '';
}
