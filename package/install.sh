#!/usr/bin/bash

if grep -q '^ID=nixos' /etc/os-release 2>/dev/null; then
    echo "Running on NixOS, installation aborted"
    exit 1
fi

killall xkbflag-rs 2>/dev/null

echo "Installing application"
install -m 755 ./xkbflag-rs /usr/bin/
install ./xkbflag-rs.desktop /usr/share/applications/

echo "Installation finished."
