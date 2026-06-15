class Cliare < Formula
  desc "Audit CLIs for agent readiness, command indexes, and safe discovery behavior"
  homepage "https://github.com/modiqo/cliare"
  url "https://github.com/modiqo/cliare/archive/refs/tags/v0.1.1.tar.gz"
  sha256 "REPLACE_WITH_SHA256"
  license "Apache-2.0"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match "cliare 0.1.1", shell_output("#{bin}/cliare metadata --format text")
    assert_match "cliare.metadata.v1", shell_output("#{bin}/cliare metadata --format json")
  end
end
