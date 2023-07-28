#!/bin/sh

set -ex

# User defined list of fake systemd units
MOCK_SYSTEMD_UNITS=${MOCK_SYSTEMD_UNITS:-""}

dbus_healthy() {
	# Test to see if the dbus service is running
	dbus-send --system --print-reply \
		--dest=org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus.ListNames
}

# Wait for dbus
echo "Waiting for dbus"
until dbus_healthy >/dev/null; do
	sleep 1
done
echo "D-Bus socket ready"

# Add unit arguments if any
if [ -n "${MOCK_SYSTEMD_UNITS}" ]; then
	for u in ${MOCK_SYSTEMD_UNITS}; do
		set -- "$@" "$u"
	done
fi

# Start service passing the extra
# units as command line arguments
exec "$@"
