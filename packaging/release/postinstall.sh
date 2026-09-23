#!/bin/sh
cat <<'EOF'

Headroom is installed. As your own user (not root), start the daemon and enable a panel:

  systemctl --user daemon-reload
  systemctl --user enable --now headroom.service

  GNOME:  gnome-extensions enable headroom@headroom.github.io
          (on Wayland, log out and back in first)
  Plasma: add the "Headroom" widget to your panel

Panels also start the daemon on demand through D-Bus activation.
EOF
