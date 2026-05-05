class Recon < Formula
  desc "Versatile network reconnaissance CLI: HTTP/TLS/DNS, multi-protocol probes, and a Rhai script engine"
  homepage "https://github.com/codedeviate/recon"
  url "https://github.com/codedeviate/recon/archive/refs/tags/v0.77.3.tar.gz"
  # Replace this placeholder after creating the GitHub release tag.
  # Compute via: `curl -sL <url> | shasum -a 256`
  sha256 "REPLACE_WITH_SHA256_OF_RELEASE_TARBALL"
  license "MIT"
  head "https://github.com/codedeviate/recon.git", branch: "master"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match "recon #{version}", shell_output("#{bin}/recon --version")
    # Smoke test: --flags should list at least one known flag.
    assert_match "--header", shell_output("#{bin}/recon --flags")
  end
end
