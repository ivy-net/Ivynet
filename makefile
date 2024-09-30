.PHONY: all bnc

all: bnc

bnc: ## Build and copy ivynet to ~/bin
	@echo "Starting release build..."
	cargo build -r
	@echo "Copying ivynet to ~/bin..."
	cp target/release/ivynet ~/bin
	@echo "Script completed."
