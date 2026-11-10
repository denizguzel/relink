.PHONY: build desktop linux linux-bin package test check snaps linklibs fmt lint

TARGET := x86_64-pc-windows-msvc
EXE := target/$(TARGET)/release/relink.exe
LINKLIBS := $(CURDIR)/target/linklibs
WINDIR := dist/relink-windows

build:
	cargo xwin build --locked --release --target $(TARGET) --bin relink
	mkdir -p $(WINDIR)
	cp $(EXE) $(WINDIR)/

# Build, then replace the portable copy on the Windows Desktop and start it.
desktop: build
	scripts/deploy-desktop.sh

# Linux x86_64 for Ubuntu 20.04+ (glibc 2.31) via cargo-zigbuild. `linux-bin`
# only compiles (CI uses it too); `linux` also packages dist/release/.
LINUX := x86_64-unknown-linux-gnu
VERSION := $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
linux-bin: linklibs
	RUSTFLAGS="-L $(LINKLIBS) -C link-arg=-Wl,--allow-shlib-undefined" cargo zigbuild --locked --release --target $(LINUX).2.31 --bin relink --bin relink-helper

# All release files that can be made here (no Windows installer: Inno Setup
# runs in CI). Same script as the release workflow.
package: build linux-bin
	python3 scripts/package.py --version $(VERSION) --out dist/release \
	  --windows-exe $(EXE) \
	  --linux-bin target/$(LINUX)/release/relink --helper target/$(LINUX)/release/relink-helper

linux: linux-bin
	python3 scripts/package.py --version $(VERSION) --out dist/release \
	  --linux-bin target/$(LINUX)/release/relink --helper target/$(LINUX)/release/relink-helper

linklibs:
	@mkdir -p $(LINKLIBS)
	@for lib in xkbcommon wayland-client; do \
	  f=$$(find /usr/lib/x86_64-linux-gnu -maxdepth 1 -name "lib$$lib.so.*" | sort | head -1); \
	  [ -n "$$f" ] && ln -sf "$$f" $(LINKLIBS)/lib$$lib.so; done

test: linklibs
	RUSTFLAGS="-L $(LINKLIBS)" cargo test

snaps: linklibs
	RUSTFLAGS="-L $(LINKLIBS)" cargo build
	@mkdir -p target/snaps
	@for s in ok welcome stuck missing settling paused host-detail vpn-down connections hosts settings activity overview-details connections-technical settings-advanced help help-en zoom-125 zoom-85 stuck-en settings-en connections-en hosts-en activity-en update ok-light stuck-light stuck-en-light missing-light settling-light welcome-light paused-light host-detail-light vpn-down-light connections-light hosts-light settings-light activity-light update-light; do \
	  VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json ./target/debug/relink --snap target/snaps/$$s.png --scene $$s >/dev/null 2>&1 && echo target/snaps/$$s.png; done

check: build
	cd $(WINDIR) && ./relink.exe --check -v

fmt:
	cargo fmt

lint: linklibs
	cargo fmt --check
	RUSTFLAGS="-L $(LINKLIBS)" cargo clippy --all-targets -- -D warnings
