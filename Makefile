define wasm_bindgen
	cargo build $(2)
	wasm-bindgen \
		--target web \
		--out-dir dist \
		--no-typescript \
		target/wasm32-unknown-unknown/$(1)/wgpustein.wasm
endef

.PHONY: all
all: dist

.PHONY: clean
clean:
	rm -rfv dist/

.PHONY: dist
dist: static
	$(call wasm_bindgen,debug)

.PHONY: release-dist
release-dist: static
	$(call wasm_bindgen,release,--release)

.PHONY: static
static:
	mkdir -p dist
	cp -v src/index.html dist/
