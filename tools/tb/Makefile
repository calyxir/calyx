TARGET = tb

.PHONY: $(TARGET) | plugins
$(TARGET):
	cargo build --manifest-path Cargo.toml
	printf "../../target/debug/tb \$$@\n" > $@
	chmod u+x $@

.PHONY: plugins
plugins: build_plugins.py
	$(shell which python3 || which python || which pypy3 || which pypy)	$<
