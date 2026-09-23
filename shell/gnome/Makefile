UUID := headroom@headroom.github.io
BUILD := build
BUNDLE := $(BUILD)/$(UUID).shell-extension.zip
INSTALL_DIR := $(or $(XDG_DATA_HOME),$(HOME)/.local/share)/gnome-shell/extensions/$(UUID)
STYLESHEETS := stylesheet.css stylesheet-light.css stylesheet-dark.css
SOURCES := $(shell find src -name '*.js') extension.js prefs.js metadata.json

.PHONY: all styles pack install uninstall lint test format mock devkit sample clean

all: pack

$(STYLESHEETS) &: $(wildcard styles/template/*.css) styles/tokens.json styles/build.py
	python3 styles/build.py

styles: $(STYLESHEETS)

$(BUNDLE): $(STYLESHEETS) $(SOURCES) $(wildcard icons/*)
	mkdir -p $(BUILD)
	gnome-extensions pack --force --out-dir=$(BUILD) \
		--extra-source=src --extra-source=icons \
		--extra-source=stylesheet-light.css --extra-source=stylesheet-dark.css .

pack: $(BUNDLE)

install: $(BUNDLE)
	gnome-extensions install --force $(BUNDLE)

uninstall:
	rm -rf $(INSTALL_DIR)

lint:
	npx eslint .
	npx prettier --check .

test:
	gjs -m tests/run.js

format:
	npx prettier --write .

mock:
	python3 dev/mock-daemon.py --scenario $(or $(SCENARIO),full)

devkit: $(BUNDLE)
	dev/devkit.sh $(BUNDLE)

sample:
	python3 dev/mock-daemon.py --dump > dev/sample-state.json

clean:
	rm -rf $(BUILD) $(STYLESHEETS)
