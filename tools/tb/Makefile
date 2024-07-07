TARGET = tb

.PHONY: $(TARGET)
$(TARGET): plugins
	cargo build --manifest-path Cargo.toml
	printf "../../target/debug/tb \$$@\n" > $@
	chmod u+x $@

.PHONY: plugins
plugins:
	$(shell which python3 || which python || which pypy3 || which pypy)	build_plugins.py
