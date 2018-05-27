with import <nixpkgs> {};
mkShell {
  buildInputs = [rustChannels.stable.rust inotify-tools];
}
