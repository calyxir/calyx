TARGET = tb

.PHONY: $(TARGET)
$(TARGET):
	cargo build --manifest-path Cargo.toml
	printf "../../target/debug/tb \$$@\n" > $@
	chmod u+x $@
