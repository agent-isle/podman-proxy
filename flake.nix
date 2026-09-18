{
  description = "podman-proxy - a Unix socket proxy enforcing a sandbox mount policy on Podman";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      nixpkgsFor = forAllSystems (system: nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgsFor.${system};
        in
        {
          podman-proxy = pkgs.rustPlatform.buildRustPackage {
            pname = "podman-proxy";
            version = "0.1.0";
            src = ./.;
            cargoHash = "sha256-WgFYAfys5lqGEElLtiGHHCijHOLzODsulqdJVwzwnNE=";
          };
          default = self.packages.${system}.podman-proxy;
        }
      );

      devShells = forAllSystems (system:
        let
          pkgs = nixpkgsFor.${system};
        in
        {
          default = pkgs.mkShell {
            name = "podman-proxy";
            packages = with pkgs; [
              rustc
              cargo
              clippy
              rustfmt
              pandoc
              panache
              lychee
              cacert
            ];
            shellHook = ''
              git config core.hooksPath githooks 2>/dev/null || true
              alias build-docs='./scripts/build-docs.sh'
              alias build-ai-dev-env='nix print-dev-env > ./scripts/ai-dev-env.sh && echo "export SSL_CERT_FILE=\"$NIX_SSL_CERT_FILE\"" >> ./scripts/ai-dev-env.sh'
            '';
          };
        }
      );
    };
}
