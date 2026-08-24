{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  inputsFrom = with pkgs; [
    llvmPackages.bintools
    rustc

    python3
    (pkgs.python3.withPackages (python-pkgs: [
      python-pkgs.python-dotenv
      python-pkgs.github3-py
    ]))
  ];

  buildInputs = with pkgs; [
    llvmPackages.bintools
    # gcc

    rustc
    cargo
    rust-analyzer
    clippy
    cargo-audit
    cargo-crev
    # cargo-deb # build deb
    # cargo-deps # dependency graph
    rustfmt

    python3
    python3Packages.pip
    python3Packages.virtualenv
  ];

  packages = with pkgs; [
    pkg-config
    llvmPackages.bintools
    rustc
  ];

  shellHook = ''
    # Optional: Print a message when entering the environment
    echo "Entering Rust development environment..."

    # Optional: Set up Rust-specific environment variables
    export RUST_LOG=debug
    export RUST_BACKTRACE=1

    # Define a directory for your virtual environment
    VENV_DIR=".venv"

    # Create the virtual environment if it doesn't exist
    if [ ! -d "$VENV_DIR" ]; then
      echo "Creating Python virtual environment in $VENV_DIR..."
      python3 -m venv "$VENV_DIR"
    fi

    # Activate the virtual environment
    source "$VENV_DIR/bin/activate"

    # Install your dependencies using pip (e.g., from a requirements.txt)
    # pip install -r requirements.txt

    # You would then install dotenv and github3.py via pip here
    # pip install python-dotenv github3py

    echo "Python environment ready!"
  '';

  # Certain Rust tools won't work without this
  # This can also be fixed by using oxalica/rust-overlay and specifying the rust-src extension
  # See https://discourse.nixos.org/t/rust-src-not-found-and-other-misadventures-of-developing-rust-on-nixos/11570/3?u=samuela. for more details.
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
