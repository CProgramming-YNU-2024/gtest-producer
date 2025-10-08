# Setup GTest Action

A composite GitHub Action that downloads and installs pre-built GTest from the `gtest-producer` repository.

## Features

- ✨ Fast installation - downloads pre-built GTest instead of building from source
- 🎯 Flexible versioning - use latest or specify a version
- 🔧 Configurable source repository
- 📦 Simple to use - just one step in your workflow

## Usage

### Basic Usage (Latest Version)

```yaml
- name: Setup GTest
  uses: CProgramming-YNU-2024/gtest-producer/.github/actions/setup-gtest@main
```

### Specify Version

```yaml
- name: Setup GTest
  uses: CProgramming-YNU-2024/gtest-producer/.github/actions/setup-gtest@main
  with:
    gtest-version: 'build-20251008.9'  # Or 'v1.0.0', etc.
```

### Use Custom Producer Repository

```yaml
- name: Setup GTest
  uses: CProgramming-YNU-2024/gtest-producer/.github/actions/setup-gtest@main
  with:
    producer-repo: 'your-org/your-gtest-producer'
    gtest-version: 'latest'
```

## Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `gtest-version` | Version tag or "latest" to download | No | `latest` |
| `producer-repo` | Repository containing GTest releases (owner/repo format) | No | `CProgramming-YNU-2024/gtest-producer` |

## Outputs

| Output | Description | Value |
|--------|-------------|-------|
| `gtest-path` | Path where GTest was installed | `/usr/local/gtest-libs` |
| `include-path` | Path to GTest include directory | `/usr/local/gtest-libs/include` |
| `lib-path` | Path containing GTest library files | `/usr/local/gtest-libs` |

## Complete Example

Here's a complete example of using this action in a workflow:

```yaml
name: Test with GTest

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
      - name: Checkout code
        uses: actions/checkout@v4
      
      - name: Setup GTest
        uses: CProgramming-YNU-2024/gtest-producer/.github/actions/setup-gtest@main
        id: gtest
      
      - name: Build and Test
        run: |
          echo "GTest installed at: ${{ steps.gtest.outputs.gtest-path }}"
          
          # Compile your test
          g++ -std=c++17 \
            -I${{ steps.gtest.outputs.include-path }} \
            your_test.cpp \
            ${{ steps.gtest.outputs.lib-path }}/libgtest.a \
            ${{ steps.gtest.outputs.lib-path }}/libgtest_main.a \
            -pthread \
            -o test_runner
          
          # Run tests
          ./test_runner
```

## Using with CMake

You can use this action with CMake's `find_package`:

```yaml
- name: Setup GTest
  uses: CProgramming-YNU-2024/gtest-producer/.github/actions/setup-gtest@main
  id: gtest

- name: Configure with CMake
  run: |
    cmake -B build \
      -DGTEST_ROOT=${{ steps.gtest.outputs.gtest-path }} \
      -DGTEST_INCLUDE_DIR=${{ steps.gtest.outputs.include-path }} \
      -DGTEST_LIBRARY=${{ steps.gtest.outputs.lib-path }}/libgtest.a \
      -DGTEST_MAIN_LIBRARY=${{ steps.gtest.outputs.lib-path }}/libgtest_main.a
```

Or in your `CMakeLists.txt`:

```cmake
find_package(GTest QUIET)

if (GTest_FOUND)
  message(STATUS "Local GTest found")
  # Use local GTest
else()
  message(STATUS "Local GTest not found, falling back...")
  # FetchContent or ExternalProject_Add logic here
endif()
```

## Requirements

- Runs on `ubuntu-latest` or compatible Linux runners
- Requires `curl` and `tar` (pre-installed on GitHub runners)
- Requires `sudo` access for installation to `/usr/local/`

## How It Works

1. Downloads the pre-built GTest tarball from the specified producer repository
2. Extracts it to `/usr/local/gtest-libs/`
3. Makes the headers and libraries available for your build

This significantly speeds up workflow execution compared to building GTest from source, which can take several minutes.

## Performance

- **Download time**: ~1-2 seconds
- **Extraction time**: <1 second
- **Total setup time**: ~2-3 seconds

Compare this to building GTest from source which typically takes 2-5 minutes!
