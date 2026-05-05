class ReconImpersonate < Formula
  desc "recon with browser TLS+H2 fingerprint impersonation (BoringSSL via rquest)"
  homepage "https://github.com/codedeviate/recon"
  url "https://github.com/codedeviate/recon/archive/refs/tags/v0.77.3.tar.gz"
  # Replace this placeholder after creating the GitHub release tag.
  # Compute via: `curl -sL <url> | shasum -a 256`
  sha256 "REPLACE_WITH_SHA256_OF_RELEASE_TARBALL"
  license "MIT"
  head "https://github.com/codedeviate/recon.git", branch: "master"

  # Conflicts with the lean `recon` formula because both install a binary
  # named `recon` into HOMEBREW_PREFIX/bin. Use one or the other.
  conflicts_with "recon",
    because: "both install the `recon` binary"

  depends_on "rust" => :build
  depends_on "cmake" => :build # BoringSSL build prerequisite

  def install
    system "cargo", "install",
           "--features", "impersonate",
           *std_cargo_args(path: ".")
  end

  test do
    assert_match "recon #{version}", shell_output("#{bin}/recon --version")
    assert_match "TLS-impersonation", shell_output("#{bin}/recon --version")
    # Confirm a profile name is accepted at parse time.
    assert_match "recon",
                 shell_output("#{bin}/recon --impersonate chrome_131 --version")
  end
end
