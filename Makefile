.PHONY: serve build

serve:
	uv run mkdocs serve

build:
	uv run mkdocs build
