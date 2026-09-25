#!/usr/bin/env python3
import argparse
import json
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib

sys.path.insert(0, str(Path(__file__).resolve().parent))
from mock_accounts import recovery
from mock_common import iso
from mock_methods import APP_VERSION, check_result, diagnostics, spend_report
from mock_registry import PROVIDERS
from mock_scenarios import SCENARIOS, base_state, build, finished, initial_settings
from mock_settings import merge_patch, normalized_settings, reset_settings

BUS_NAME = "io.github.daniarjabagin.Headroom"
OBJECT_PATH = "/io/github/daniarjabagin/Headroom"
INTERFACE = "io.github.daniarjabagin.Headroom1"
INTERFACE_XML = f"""
<node>
  <interface name="{INTERFACE}">
    <method name="GetState"><arg type="s" name="state" direction="out"/></method>
    <method name="ListProviders"><arg type="s" name="providers" direction="out"/></method>
    <method name="Refresh"><arg type="s" name="account_id" direction="in"/></method>
    <method name="RefreshNow"/>
    <method name="Rescan"/>
    <method name="CheckForUpdates"><arg type="s" name="result" direction="out"/></method>
    <method name="RestoreAccounts"><arg type="s" name="provider" direction="in"/></method>
    <method name="DismissAccount"><arg type="s" name="account_id" direction="in"/></method>
    <method name="GetSettings"><arg type="s" name="settings" direction="out"/></method>
    <method name="SetSettings"><arg type="s" name="json" direction="in"/></method>
    <method name="UpdateSettings"><arg type="s" name="patch" direction="in"/></method>
    <method name="ResetSettings"/>
    <method name="GetSpend">
      <arg type="s" name="query" direction="in"/>
      <arg type="s" name="result" direction="out"/>
    </method>
    <method name="GetDiagnostics"><arg type="s" name="report" direction="out"/></method>
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
CHECK_MS = 1500
CHECK_OUTCOMES = ("up_to_date", "available", "failed", "rate_limited", "unsupported")
REFRESH_NOW_MS = 1500
SAMPLE_TIME = datetime(2026, 9, 23, 10, 0, tzinfo=timezone.utc)
STRING_REPLIES = {"GetState", "ListProviders", "GetSettings", "GetSpend", "GetDiagnostics"}


def ordered(accounts, order):
    rank = {account_id: index for index, account_id in enumerate(order)}
    return sorted(accounts, key=lambda entry: rank.get(entry["id"], len(rank)))


class MockDaemon:
    def __init__(self, scenario, interval, check_outcome):
        self.scenario = scenario
        self.interval = interval
        self.check_outcome = check_outcome
        self.started_at = datetime.now(timezone.utc)
        self.checked_at = self.started_at - timedelta(hours=3)
        self.hidden = set()
        self.dismissed = set()
        self.labels = {}
        self.order = []
        self.settings = initial_settings(scenario)
        self.refreshing = set()
        self.refreshed_at = None
        self.connection = None

    def shown(self, state):
        return [entry for entry in state.get("accounts", []) if entry["id"] not in self.dismissed]

    def account_ids(self):
        return [account["id"] for account in self.shown(base_state(self.scenario))]

    def decorate(self, account):
        hidden_windows = self.settings["display"]["hidden_windows"].get(account["id"], [])
        account["hidden"] = account["id"] in self.hidden
        account["label"] = self.labels.get(account["id"], account["label"])
        for window in account["windows"]:
            window["hidden"] = window["id"] in hidden_windows
        account["recovery"] = recovery(account)
        if account["id"] in self.refreshing:
            account["status"] = "refreshing"
        if account["status"] == "signed_out":
            return
        if self.refreshed_at and account["status"] == "stale":
            account["status"] = "fresh"
        if self.refreshed_at and account["status"] == "fresh":
            account["updated_at"] = iso(self.refreshed_at)

    def live_state(self):
        state = base_state(self.scenario)
        state["accounts"] = self.shown(state)
        for account in state["accounts"]:
            self.decorate(account)
        state["accounts"] = ordered(state["accounts"], self.order)
        state["app_version"] = APP_VERSION
        state["update_check"] = {"checked_at": iso(self.checked_at)}
        if not self.settings["updates"]["check"]:
            state["update"] = None
            state["update_check"] = None
        if self.refreshed_at and state.get("last_success_at"):
            state["last_success_at"] = iso(self.refreshed_at)
        return state

    def state(self):
        return json.dumps(finished(self.live_state(), self.settings, self.pinned()))

    def pinned(self):
        pin = self.settings["headline"]
        return (pin.get("account_id"), pin.get("window")) if pin["mode"] == "pinned" else None

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

    def check_for_updates(self, invocation):
        if self.check_outcome == "unsupported":
            invocation.return_dbus_error("org.freedesktop.DBus.Error.NotSupported", "update checks are off")
            return
        if not self.settings["updates"]["check"]:
            self.reply(invocation, json.dumps({"status": "disabled", "checked_at": None, "version": None}))
            return
        GLib.timeout_add(CHECK_MS, self.finish_check, invocation)

    def finish_check(self, invocation):
        now = datetime.now(timezone.utc)
        if self.check_outcome in ("up_to_date", "available"):
            self.checked_at = now
            self.emit()
        self.reply(invocation, json.dumps(check_result(self.check_outcome, iso(self.checked_at), now)))
        return GLib.SOURCE_REMOVE

    def reply(self, invocation, text):
        invocation.return_value(GLib.Variant("(s)", (text,)))

    def set_order(self, ids):
        if len(set(ids)) != len(ids):
            raise ValueError("duplicate account id in order")
        rest = [account_id for account_id in (self.order or self.account_ids()) if account_id not in ids]
        self.order = list(ids) + rest

    def dismiss(self, account_id):
        if account_id not in self.account_ids():
            raise ValueError(f"unknown account {account_id}")
        self.dismissed.add(account_id)

    def restore(self, provider):
        self.dismissed = {account_id for account_id in self.dismissed if not account_id.startswith(f"{provider}:")}

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

    def string_reply(self, method, args):
        if method == "GetState":
            return self.state()
        if method == "ListProviders":
            return json.dumps({"version": 1, "providers": PROVIDERS})
        if method == "GetSettings":
            return json.dumps(self.settings)
        if method == "GetSpend":
            usage = base_state(self.scenario)["usage_source"]
            return json.dumps(spend_report(args[0], usage, datetime.now().date()))
        now = datetime.now(timezone.utc)
        return json.dumps(diagnostics(self.live_state()["accounts"], self.settings, self.started_at, now))

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
        elif method == "ResetSettings":
            self.settings = reset_settings(self.settings)
        elif method == "DismissAccount":
            self.dismiss(args[0])
        elif method == "RestoreAccounts":
            self.restore(args[0])
        self.emit()

    def on_method(self, _connection, _sender, _path, _interface, method, parameters, invocation):
        args = parameters.unpack()
        print(f"{method}{args}", flush=True)
        if method == "CheckForUpdates":
            self.check_for_updates(invocation)
            return
        try:
            if method in STRING_REPLIES:
                self.reply(invocation, self.string_reply(method, args))
                return
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
    parser.add_argument("--check", choices=CHECK_OUTCOMES, default="up_to_date", help="CheckForUpdates answer")
    parser.add_argument("--dump", action="store_true", help="print a state at a fixed sample time and exit")
    args = parser.parse_args()
    if args.dump:
        print(json.dumps(build(args.scenario, SAMPLE_TIME), indent=2))
        return
    MockDaemon(args.scenario, args.interval, args.check).run()


if __name__ == "__main__":
    main()
