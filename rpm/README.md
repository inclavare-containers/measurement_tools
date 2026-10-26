# Native RPM builds

On a Linux **x86_64 or aarch64** host with Git and Docker, check out the desired
release tag or source revision, then run from the repository root:

```sh
sudo bash rpm/build-rpm.sh al8   # Alibaba Cloud Linux 3
sudo bash rpm/build-rpm.sh an23  # Anolis OS 23
```

The scripts build the current tracked source files in a container of the same
architecture. They are native builds, not cross-compilation. Output:
`rpm-output/<distribution>/<architecture>/{RPMS,SRPMS}/`.

`BASE_IMAGE` overrides the distribution image; `OUTPUT_DIR` overrides the output
directory. Cargo caches use Docker volumes scoped by distribution and architecture.
The native RPM workflow builds both architectures and installs/runs the packages.

For a native build without Docker, install the spec's build dependencies plus
`rpmdevtools`, `rsync` and Rust/Cargo, then run `bash build_rpm.sh`.
The version defaults to `measurement_tool.spec`; both supported architectures
use the corresponding GNU Rust target. Container smoke tests skip installation
of the separately packaged Attestation Agent dependency.
