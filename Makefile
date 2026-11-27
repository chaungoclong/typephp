.PHONY: help test build check blog blog-bundle rest-api clean docker-build docker-test docker-shell docker-blog

PORT ?= 18082

help: ## Show available make targets
	@grep -E '^[a-zA-Z0-9_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'

test: ## Run all Rust & MC/DC tests
	cargo test

build: ## Build tphpc release binary
	cargo build --release

check: ## Type-check blog_app without emitting PHP
	cargo run -- examples/blog_app/public/index.tphp --check

blog: ## Compile PSR-7 microfw + Blog App (with PHPStan Level 9)
	cargo run -- examples/blog_app/public/index.tphp

blog-bundle: ## Compile Blog App into a single minified PHP file
	cargo run -- examples/blog_app/public/index.tphp --bundle --minify -o /tmp/blog.min.php && php -l /tmp/blog.min.php

rest-api: ## Compile SQLite REST API example
	cargo run -- examples/rest_api/index.tphp -o examples/rest_api/index.php

docker-build: ## Build Docker dev image
	docker compose build dev

docker-test: ## Run cargo test inside Docker dev container
	docker compose run --rm dev cargo test

docker-shell: ## Open bash shell inside Docker dev container
	docker compose run --rm dev bash

docker-blog: ## Compile and serve Blog App in Docker (http://127.0.0.1:$(PORT))
	TPHP_PORT_8082=$(PORT) docker compose run --rm --service-ports dev sh -c "cargo run -- examples/blog_app/public/index.tphp && php -S 0.0.0.0:8082 examples/blog_app/public/index.php"

clean: ## Remove temporary build/test artifacts
	rm -f /tmp/blog.min.php /tmp/blog.db
