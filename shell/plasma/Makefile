PLUGIN_ID := io.github.headroom.plasmoid
QT_BIN := /usr/lib/qt6/bin
QML := $(QT_BIN)/qml
QMLLINT := $(QT_BIN)/qmllint
QMLFORMAT := $(QT_BIN)/qmlformat
QMLTESTRUNNER := $(QT_BIN)/qmltestrunner
STUBS := dev/preview/stubs
QML_SOURCES := $(shell find package dev tests -name '*.qml')
MOCK := ../gnome/dev/mock-daemon.py
OUT ?= $(or $(XDG_RUNTIME_DIR),/tmp)/headroom-plasma-preview/shot
SCENARIO ?= ready
STATE ?=
PREVIEW_FLAGS ?=
PREVIEW_ENV := QML_XHR_ALLOW_FILE_READ=1 QT_FORCE_STDERR_LOGGING=1 QT_SCALE_FACTOR=2

.PHONY: install uninstall lint format test preview state mock

install:
	kpackagetool6 --type Plasma/Applet --upgrade package || kpackagetool6 --type Plasma/Applet --install package

uninstall:
	kpackagetool6 --type Plasma/Applet --remove $(PLUGIN_ID)

lint:
	$(QMLLINT) --max-warnings 0 -I $(STUBS) $(QML_SOURCES)
	@for file in $(QML_SOURCES); do $(QMLFORMAT) $$file | diff -q $$file - >/dev/null || { echo "not qmlformat-clean: $$file"; exit 1; }; done

format:
	$(QMLFORMAT) -i $(QML_SOURCES)

test:
	python3 tests/check_messages.py
	QML_XHR_ALLOW_FILE_READ=1 QT_QPA_PLATFORM=offscreen $(QMLTESTRUNNER) -input tests

preview:
	mkdir -p $(dir $(OUT))
	$(PREVIEW_ENV) $(QML) -platform offscreen:configfile=dev/preview/screen.json -I $(STUBS) dev/preview/Preview.qml -- \
		--out $(OUT) --scenario $(SCENARIO) $(if $(STATE),--state $(abspath $(STATE))) $(PREVIEW_FLAGS)

state:
	python3 $(MOCK) --dump --scenario $(or $(NAME),full)

mock:
	python3 $(MOCK) --scenario $(or $(NAME),full)
