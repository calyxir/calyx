examples/data.json: examples/config.json
	cat examples/config.json | ./examples/collect.py | jq > examples/data.json

clean:
	rm examples/data.json
