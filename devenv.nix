# Development shell for Cast Iron. Enter it with `devenv shell`, or with direnv
# through `.envrc`. It provides the same tool versions as CI:
# - Rust from rust-toolchain.toml (channel, components, wasm32 target).
# - Trunk 0.22.1, the release binary CI downloads, checked by the same SHA-256.
# - wasm-opt (binaryen) so Trunk does not download a glibc binary.
# - Node 24 and Playwright browsers built by Nix. The browser version must match
#   @playwright/test in tests/e2e/package-lock.json.
# - python3 for scripts/, perl for the CSP stamp hook.
{ pkgs, lib, ... }:

let
  trunkVersion = "0.22.1";
  # Same value as TRUNK_SHA256 in .github/workflows/ci.yml.
  trunk = pkgs.stdenv.mkDerivation {
    pname = "trunk";
    version = trunkVersion;
    src = pkgs.fetchurl {
      url = "https://github.com/trunk-rs/trunk/releases/download/v${trunkVersion}/trunk-x86_64-unknown-linux-gnu.tar.gz";
      sha256 = "2ab8d548fd5ed9cda50906f61fd8654204e7893bd52ebbfd90533862b7d71ee1";
    };
    sourceRoot = ".";
    nativeBuildInputs = [ pkgs.autoPatchelfHook ];
    buildInputs = [ pkgs.stdenv.cc.cc.lib pkgs.openssl ];
    installPhase = "install -Dm755 trunk $out/bin/trunk";
  };
  playwrightLock = builtins.fromJSON (builtins.readFile ./tests/e2e/package-lock.json);
  playwrightVersion = playwrightLock.packages."node_modules/@playwright/test".version;
in
{
  languages.rust = {
    enable = true;
    toolchainFile = ./rust-toolchain.toml;
  };

  languages.javascript = {
    enable = true;
    package = pkgs.nodejs_24;
  };

  packages = [
    trunk
    pkgs.binaryen
    pkgs.python3
    pkgs.perl
    pkgs.git
    pkgs.gh
  ];

  env = {
    PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
    PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD = "1";
    PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";
  };

  assertions = [
    {
      assertion = pkgs.playwright-driver.version == playwrightVersion;
      message = "nixpkgs Playwright ${pkgs.playwright-driver.version} does not match @playwright/test ${playwrightVersion} in tests/e2e/package-lock.json. Update devenv.lock or the npm lock so they match.";
    }
  ];

  enterTest = ''
    rustc --version | grep -F "$(sed -n 's/^channel = "\(.*\)"/\1/p' rust-toolchain.toml)"
    trunk --version | grep -F "${trunkVersion}"
    wasm-opt --version
    node --version | grep '^v24\.'
  '';
}
