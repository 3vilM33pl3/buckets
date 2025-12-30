class Buckets < Formula
  desc "Game asset and expectation management tool"
  homepage "https://github.com/3vilM33pl3/buckets"
  url "https://github.com/3vilM33pl3/buckets/releases/download/v0.3.0/buckets-macos-universal.tar.gz"
  # sha256 "" # TODO: Update this after the release is built
  version "0.3.0"

  def install
    bin.install "buckets"
  end
end
