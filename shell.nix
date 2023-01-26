let
  pkgs = import ./pinned.nix {};
in with pkgs; mkShell {
  buildInputs = [
    cargo
    clang
    llvmPackages.libclang
    rocksdb
    rust-analyzer
    rustup
  ];
  shellHooks = ''
    export CARGO_PATH=${cargo}/bin/cargo
    export RUST_ANALYZER=${rust-analyzer}/bin/rust-analyzer
    export LIBCLANG_PATH="${llvmPackages.libclang.lib}/lib"
  '';
}
