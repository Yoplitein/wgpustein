STATICS := index.html
DIST_STATICS = $(addprefix dist/, $(STATICS))

define wasm_bindgen
	cargo build $(2)
	wasm-bindgen \
		--target web \
		--out-dir dist \
		--no-typescript \
		target/wasm32-unknown-unknown/$(1)/wgpustein.wasm
endef

.PHONY: all
all: build

.PHONY: clean
clean:
	rm -rfv dist/

.PHONY: build
build: dist $(DIST_STATICS)
	$(call wasm_bindgen,debug)

.PHONY: release-build
release-build: dist $(DIST_STATICS)
	$(call wasm_bindgen,release,--release)

dist:
	mkdir -p dist

$(DIST_STATICS): dist/%: src/% dist
	cp -v $< $@

.PHONY: test
test:
	cargo test --target=$(shell rustc -vV | grep host | cut -d' ' -f2)
