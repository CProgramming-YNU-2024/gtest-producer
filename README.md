# gtest-producer

This repo simulates producing a gtest binary as a GitHub release asset. A release workflow packages `tools/gtest` and uploads it as an asset named `gtest-linux-x64.tar.gz`. The `tools/gtest` is a tiny POSIX shell script that prints a version string when run.

Use the `.github/workflows/release.yml` workflow to create a release (triggered by `workflow_dispatch`).