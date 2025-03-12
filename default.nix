{ rustPlatform
, pkg-config
, systemd
}:

rustPlatform.buildRustPackage {
  pname = "cdock-inhibit";
  version = "0.0.1";

  src = ./.;

  buildInputs = [ systemd ];
  nativeBuildInputs = [ pkg-config ];

  cargoLock = {
    lockFile = ./Cargo.lock;
  };
}
