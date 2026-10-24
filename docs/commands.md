# Commands

Common commands:

```bash
ncc check archivo.n
ncc run archivo.n
ncc emit-ir archivo.n
ncc project init
ncc doctor bins
ncc clean
ncc clean --cache
```

Release helpers:

```bash
ncc-release-v43 prepare-public-beta-layout --root .
ncc run tools/release/verify_public_beta_layout.n
ncc-release-v43 package-public-beta --root .
cd dist/public-beta/ncc
ghor public-beta
```

The release helpers are nC executables, not interpreted scripts. Build the
`ncc-release-v43` family using the [native constructor](PYTHON_RETIREMENT_V43.md#construccion)
and put its output `bin` directory on PATH. On Windows the filename is
`ncc-release-v43.exe`. Rust remains the Stage0 compiler until bootstrap equivalence
is established; generating a package does not promote Stage3.
