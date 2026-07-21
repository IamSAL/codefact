# Homebrew formula for the codefacts CLI (+ worker).
# Update `version`, url, and sha256 per release (or automate with brew bump).
class Codefacts < Formula
  desc "Codebase-knowledge engine that pushes insights to Telegram"
  homepage "https://github.com/OWNER/codefacts"
  version "0.1.0"
  license "MIT"

  # `iii` is required at runtime (the engine); `claude` (Claude Code) too.
  depends_on "iii" # provide via a tap or brew formula for the iii engine

  on_macos do
    on_arm do
      url "https://github.com/OWNER/codefacts/releases/download/v0.1.0/codefacts-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    end
    on_intel do
      url "https://github.com/OWNER/codefacts/releases/download/v0.1.0/codefacts-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/OWNER/codefacts/releases/download/v0.1.0/codefacts-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    end
    on_intel do
      url "https://github.com/OWNER/codefacts/releases/download/v0.1.0/codefacts-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    end
  end

  def install
    bin.install "codefacts"
  end

  test do
    assert_match "codefacts", shell_output("#{bin}/codefacts --help")
  end
end
