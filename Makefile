.PHONY: clean clippy

SOURCES:=$(wildcard ./src/*.rs)
BENCHES:=$(wildcard ./benches/*.rs)
TESTS:=$(wildcard ./tests/*.rs)
EXAMPLES:=$(wildcard ./examples/*.rs)
BUILD_OPTS:=--jobs $(shell nproc)

all: test build examples doc

clippy:
	rustup run nightly -- cargo build --features lints
	rustup run nightly -- cargo clippy -- -Dclippy -Wclippy_pedantic --verbose

build: $(SOURCES)
	cargo build $(BUILD_OPTS)

bench: $(SOURCES) $(BENCHES)
	multirust default nightly
	cargo bench
	multirust default stable

release: test $(SOURCES)
	cargo build --release $(BUILD_OPTS)

fmt: format

format: $(SOURCES) $(EXAMPLES) $(TESTS)
	@for f in $?; do\
		echo $$f && rustfmt $$f; \
	done

examples: $(SOURCES) $(EXAMPLES)
	@for f in $(basename $(notdir $(EXAMPLES))); do\
		cargo build --example $$f; \
	done

run: build
	cargo run

test: $(TESTS) $(SOURCES)
	cargo test

clean:
	rm -r ./target
	rm -f src/*.rs.bk

doc: $(SOURCES)
	cargo doc
