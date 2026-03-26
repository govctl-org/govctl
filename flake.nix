{
  description = "govctl – Project governance CLI for RFC, ADR, and Work Item management";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      in
      {
        packages = {
          govctl = pkgs.rustPlatform.buildRustPackage {
            pname = "govctl";
            inherit (cargoToml.package) version;

            src = pkgs.lib.cleanSource ./.;

            useFetchCargoVendor = true;
            cargoLock.lockFile = ./Cargo.lock;

            meta = {
              description = "Project governance CLI for RFC, ADR, and Work Item management";
              homepage = "https://github.com/govctl-org/govctl";
              license = pkgs.lib.licenses.mit;
              mainProgram = "govctl";
            };
          };

          default = self.packages.${system}.govctl;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.govctl ];
          packages = with pkgs; [
            rust-analyzer
            clippy
          ];
        };
      }
    );
}
