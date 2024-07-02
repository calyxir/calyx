TARGET = tb

.PHONY: $(TARGET) | plugins
$(TARGET):
	cargo build --manifest-path Cargo.toml
	printf "../../target/debug/tb \$$@\n" > $@
	chmod u+x $@

.PHONY: plugins
plugins:
	for dir in plugins/*; do \
		if [ -d "$$dir" ]; then \
			cargo build --manifest-path "$$dir/Cargo.toml" --release --target-dir "$$dir/target"; \
			for file in "$$dir/build/release/"*.{dylib,so}; do \
				[ -f "$$file" ] && mv "$$file" plugins; \
			done \
		fi \
	done
