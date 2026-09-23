#!/usr/bin/env python3
import argparse
import copy
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib

sys.path.insert(0, str(Path(__file__).resolve().parent))
from mock_state import DEFAULT_SETTINGS, PROVIDERS, SCENARIOS, build, choose_headline, iso

BUS_NAME = "io.github.headroom.Daemon"
OBJECT_PATH = "/io/github/headroom/Daemon"
INTERFACE = "io.github.headroom.Daemon1"
INTERFACE_XML = f"""
<node>
  <interface name="{INTERFACE}">
    <method name="GetState"><arg type="s" name="state" direction="out"/></method>
    <method name="ListProviders"><arg type="s" name="providers" direction="out"/></method>
    <method name="Refresh"><arg type="s" name="account_id" direction="in"/></method>
    <method name="RefreshNow"/>
    <method name="Rescan"/>
    <method name="GetSettings"><arg type="s" name="settings" direction="out"/></method>
    <method name="SetSettings"><arg type="s" name="json" direction="in"/></method>
    <method name="UpdateSettings"><arg type="s" name="patch" direction="in"/></method>
    <method name="SetAccountLabel">
      <arg type="s" name="account_id" direction="in"/>
      <arg type="s" name="label" direction="in"/>
    </method>
    <method name="SetAccountOrder"><arg type="as" name="ids" direction="in"/></method>
    <method name="SetAccountHidden">
      <arg type="s" name="account_id" direction="in"/>
      <arg type="b" name="hidden" direction="in"/>
    </method>
    <signal name="StateChanged"><arg type="s" name="state"/></signal>
    <signal name="OpenRequested"/>
  </interface>
</node>
"""
REFRESH_SECONDS = 2
REFRESH_NOW_MS = 1500
SAMPLE_TIME = datetime(2026, 9, 23, 10, 0, tzinfo=timezone.utc)


def merged(defaults, raw):
    if not isinstance(defaults, dict) or not isinstance(raw, dict):
        return copy.deepcopy(raw if raw is not None and type(raw) is type(defaults) else defaults)
    if not defaults:
        return copy.deepcopy(raw)
    return {key: merged(value, raw.get(key)) for key, value in defaults.items()}


def merge_patch(target, patch):
    if not isinstance(patch, dict):
        return copy.deepcopy(patch)
    result = copy.deepcopy(target) if isinstance(target, dict) else {}
    for key, value in patch.items():
        if value is None:
            result.pop(key, None)
        else:
            result[key] = merge_patch(result.get(key), value)
    return result


def normalized_settings(raw):
    settings = merged(DEFAULT_SETTINGS, raw)
    headline = raw.get("headline") if isinstance(raw.get("headline"), dict) else {}
    if headline.get("mode") == "pinned" and headline.get("account_id") and headline.get("window"):
        settings["headline"] = {"mode": "pinned", "account_id": headline["account_id"], "window": headline["window"]}
    else:
        settings["headline"] = {"mode": "auto"}
    return settings


def ordered(accounts, order):
    rank = {account_id: index for index, account_id in enumerate(order)}
    return sorted(accounts, key=lambda entry: rank.get(entry["id"], len(rank)))


class MockDaemon:
    def __init__(self, scenario, interval):
        self.scenario = scenario
        self.interval = interval
        self.hidden = set()
        self.labels = {}
        self.order = []
        self.settings = copy.deepcopy(DEFAULT_SETTINGS)
        self.refreshing = set()
        self.refreshed_at = None
        self.connection = None

    def account_ids(self):
        return [account["id"] for account in build(self.scenario).get("accounts", [])]

    def decorate(self, account):
        hidden_windows = self.settings["display"]["hidden_windows"].get(account["id"], [])
        account["hidden"] = account["id"] in self.hidden
        account["label"] = self.labels.get(account["id"], account["label"])
        for window in account["windows"]:
            window["hidden"] = window["id"] in hidden_windows
        if account["status"] == "signed_out":
            return
        if account["id"] in self.refreshing:
            account["status"] = "refreshing"
        elif self.refreshed_at and account["status"] == "stale":
            account["status"] = "fresh"
        if self.refreshed_at and account["status"] == "fresh":
            account["updated_at"] = iso(self.refreshed_at)

    def state(self):
        state = build(self.scenario)
        preferred = state["headline"] and (state["headline"]["account_id"], state["headline"]["window"])
        for account in state.get("accounts", []):
            self.decorate(account)
        state["accounts"] = ordered(state.get("accounts", []), self.order)
        pin = self.settings["headline"]
        pinned = (pin.get("account_id"), pin.get("window")) if pin["mode"] == "pinned" else None
        state["headline"] = choose_headline(state["accounts"], pinned, preferred)
        state["display"] = self.settings["display"]
        if self.refreshed_at and state.get("last_success_at"):
            state["last_success_at"] = iso(self.refreshed_at)
        return json.dumps(state)

    def emit(self):
        if self.connection:
            self.connection.emit_signal(None, OBJECT_PATH, INTERFACE, "StateChanged", GLib.Variant("(s)", (self.state(),)))
        return GLib.SOURCE_CONTINUE

    def refresh(self, account_id):
        self.refreshing = set(self.account_ids() if account_id == "" else [account_id])
        self.emit()
        GLib.timeout_add_seconds(REFRESH_SECONDS, self.finish_refresh)

    def finish_refresh(self):
        self.refreshing = set()
        self.emit()
        return GLib.SOURCE_REMOVE

    def refresh_now(self):
        self.refreshing = set(self.account_ids())
        self.emit()
        GLib.timeout_add(REFRESH_NOW_MS, self.finish_refresh_now)

    def finish_refresh_now(self):
        self.refreshed_at = datetime.now(timezone.utc)
        return self.finish_refresh()

    def set_order(self, ids):
        if len(set(ids)) != len(ids):
            raise ValueError("duplicate account id in order")
        rest = [account_id for account_id in (self.order or self.account_ids()) if account_id not in ids]
        self.order = list(ids) + rest

    def set_settings(self, text):
        raw = json.loads(text)
        if not isinstance(raw, dict):
            raise ValueError("settings must be a JSON object")
        self.settings = normalized_settings(raw)

    def update_settings(self, text):
        patch = json.loads(text)
        if not isinstance(patch, dict):
            raise ValueError("settings patch must be a JSON object")
        self.settings = normalized_settings(merge_patch(self.settings, patch))

    def apply(self, method, args):
        if method == "Refresh":
            self.refresh(args[0])
            return
        if method == "RefreshNow":
            self.refresh_now()
            return
        if method == "SetAccountHidden":
            (self.hidden.add if args[1] else self.hidden.discard)(args[0])
        elif method == "SetAccountLabel":
            self.labels[args[0]] = args[1].strip() or None
        elif method == "SetAccountOrder":
            self.set_order(args[0])
        elif method == "SetSettings":
            self.set_settings(args[0])
        elif method == "UpdateSettings":
            self.update_settings(args[0])
        self.emit()

    def on_method(self, _connection, _sender, _path, _interface, method, parameters, invocation):
        args = parameters.unpack()
        print(f"{method}{args}", flush=True)
        if method == "GetState":
            invocation.return_value(GLib.Variant("(s)", (self.state(),)))
            return
        if method == "ListProviders":
            invocation.return_value(GLib.Variant("(s)", (json.dumps({"version": 1, "providers": PROVIDERS}),)))
            return
        if method == "GetSettings":
            invocation.return_value(GLib.Variant("(s)", (json.dumps(self.settings),)))
            return
        try:
            self.apply(method, args)
        except ValueError as error:
            invocation.return_dbus_error("org.freedesktop.DBus.Error.InvalidArgs", str(error))
            return
        invocation.return_value(None)

    def on_bus_acquired(self, connection, _name):
        self.connection = connection
        info = Gio.DBusNodeInfo.new_for_xml(INTERFACE_XML).interfaces[0]
        connection.register_object(OBJECT_PATH, info, self.on_method, None, None)
        GLib.timeout_add_seconds(self.interval, self.emit)

    def run(self):
        Gio.bus_own_name(
            Gio.BusType.SESSION,
            BUS_NAME,
            Gio.BusNameOwnerFlags.NONE,
            self.on_bus_acquired,
            lambda *_: print(f"owning {BUS_NAME} ({self.scenario})", flush=True),
            lambda *_: sys.exit(f"could not own {BUS_NAME}; is another daemon running?"),
        )
        GLib.MainLoop().run()


def main():
    parser = argparse.ArgumentParser(description="Serve sample Headroom states on the session bus.")
    parser.add_argument("--scenario", choices=sorted(SCENARIOS), default="full")
    parser.add_argument("--interval", type=int, default=30, help="seconds between StateChanged signals")
    parser.add_argument("--dump", action="store_true", help="print a state at a fixed sample time and exit")
    args = parser.parse_args()
    if args.dump:
        print(json.dumps(build(args.scenario, SAMPLE_TIME), indent=2))
        return
    MockDaemon(args.scenario, args.interval).run()


if __name__ == "__main__":
    main()
