# Setup libvterm Action

This GitHub Action downloads and installs pre-built libvterm from the gtest-producer repository releases.

## Usage

```yaml
steps:
  - name: Setup libvterm
    uses: CProgramming-YNU-2024/gtest-producer/.github/actions/setup-libvterm@main
    id: libvterm

  - name: Use libvterm
    run: |
      echo "libvterm installed at: ${{ steps.libvterm.outputs.libvterm-path }}"
      echo "Include path: ${{ steps.libvterm.outputs.include-path }}"
      echo "Lib path: ${{ steps.libvterm.outputs.lib-path }}"
```

## Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `libvterm-version` | Version tag or "latest" | `latest` |
| `producer-repo` | Repository containing releases | `CProgramming-YNU-2024/gtest-producer` |
| `retry-count` | Number of download retries | `5` |
| `retry-delay` | Delay between retries (seconds) | `2` |

## Outputs

| Output | Description |
|--------|-------------|
| `libvterm-path` | Path where libvterm was installed |
| `include-path` | Path to libvterm include directory |
| `lib-path` | Path containing libvterm library files |

## CMake Integration

After running this action, you can use `find_package(Libvterm)` in your CMakeLists.txt:

```cmake
find_package(Libvterm REQUIRED)
target_link_libraries(your_target Libvterm::Libvterm)
```
