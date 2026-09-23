#!/usr/bin/env python3
import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib

sys.path.insert(0, str(Path(__file__).resolve().parent))
from mock_state import SCENARIOS, build

BUS_NAME = "io.github.headroom.Daemon"
OBJECT_PATH = "/io/github/headroom/Daemon"
INTERFACE = "io.github.headroom.Daemon1"
INTERFACE_XML = f"""
<node>
  <interface name="{INTERFACE}">
    <method name="GetState"><arg type="s" name="state" direction="out"/></method>
    <method name="Refresh"><arg type="s" name="account_id" direction="in"/></method>
    <method name="GetSettings"><arg type="s" name="settings" direction="out"/></method>
    <method name="SetSettings"><arg type="s" name="json" direction="in"/></method>
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
SAMPLE_TIME = datetime(2026, 9, 23, 10, 0, tzinfo=timezone.utc)


class MockDaemon:
    def __init__(self, scenario, interval):
        self.scenario = scenario
        self.interval = interval
        self.hidden = set()
        self.labels = {}
        self.refreshing = set()
        self.connection = None

    def state(self):
        state = build(self.scenario)
        for account in state.get("accounts", []):
            account["hidden"] = account["id"] in self.hidden
            account["label"] = self.labels.get(account["id"], account["label"])
            if account["id"] in self.refreshing and account["status"] != "signed_out":
                account["status"] = "refreshing"
        return json.dumps(state)

    def emit(self):
        if self.connection:
            self.connection.emit_signal(None, OBJECT_PATH, INTERFACE, "StateChanged", GLib.Variant("(s)", (self.state(),)))
        return GLib.SOURCE_CONTINUE

    def refresh(self, account_id):
        ids = [account["id"] for account in build(self.scenario).get("accounts", [])]
        self.refreshing = set(ids if account_id == "" else [account_id])
        self.emit()
        GLib.timeout_add_seconds(REFRESH_SECONDS, self.finish_refresh)

    def finish_refresh(self):
        self.refreshing = set()
        self.emit()
        return GLib.SOURCE_REMOVE

    def on_method(self, _connection, _sender, _path, _interface, method, parameters, invocation):
        args = parameters.unpack()
        print(f"{method}{args}", flush=True)
        if method == "GetState":
            invocation.return_value(GLib.Variant("(s)", (self.state(),)))
            return
        if method == "GetSettings":
            invocation.return_value(GLib.Variant("(s)", ("{}",)))
            return
        if method == "Refresh":
            self.refresh(args[0])
        elif method == "SetAccountHidden":
            (self.hidden.add if args[1] else self.hidden.discard)(args[0])
            self.emit()
        elif method == "SetAccountLabel":
            self.labels[args[0]] = args[1]
            self.emit()
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
