# Cargo Build Optimization Note

## Before optimization

- Clean build: 199.8s
- Incremental build: 269.9s

## After optimization

- Clean build: 79.19s
- Incremental build: 65.07s

## Methods

### 1. `cranelift`

```bash
rustup component add rustc-codegen-cranelift-preview --toolchain nightly
```

```toml .cargo/config.toml
[unstable]
codegen-backend = true

[profile.dev]
codegen-backend = "cranelift"
```

### 2. `mold`

```bash
sudo pacman -S mold
```

```toml .cargo/config.toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

### 3. `sccache`

```bash
cargo binstall sccache
```

```toml .cargo/config.toml
[build]
rustc-wrapper = "sccache"
```

### 4. `split-debuginfo`

```toml .cargo/config.toml
[profile.dev]
split-debuginfo = "unpacked"
```
