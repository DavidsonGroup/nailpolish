.PHONY: serve build

serve:
	uv run mkdocs serve

build:
	uv run mkdocs build
	find docs/ -name "*.md" -print0 | sort -z | xargs -0 cat > site/llms.md
