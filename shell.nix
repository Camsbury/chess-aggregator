let
  pkgs = import ./pinned.nix {};
in with pkgs; mkShell {
  buildInputs = [
    cargo
    clang
    clippy
    llvmPackages.libclang
    rocksdb
    linuxPackages.perf
    rust-analyzer
    rustc
    # rustup
  ];
  shellHooks = ''
    export CARGO_PATH=${cargo}/bin/cargo
    export RUST_ANALYZER=${rust-analyzer}/bin/rust-analyzer
    # export RUSTFMT_PATH=${rustup}/bin/rustfmt
    export LIBCLANG_PATH="${llvmPackages.libclang.lib}/lib"
  '';
}
