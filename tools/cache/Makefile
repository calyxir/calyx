.PHONY: cache.futil
cache.futil: src/main.rs resources/cache_template.futil
	cargo run > $@

.PHONY: clean
clean:
	rm -f cache.futil 
