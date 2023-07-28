#!/bin/sh

set -e

# User defined list of fake systemd units
MOCK_SYSTEMD_UNITS=${MOCK_SYSTEMD_UNITS:-""}

dbus_healthy() {
	# Test to see if the dbus service is running
	dbus-send --system --print-reply \
		--dest=org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus.ListNames
}

systemd_method() {
	method=$1
	shift
	dbus-send --system --print-reply \
		--dest=org.freedesktop.systemd1 \
		/org/freedesktop/systemd1 \
		"org.freedesktop.systemd1.Manager.$method" "$@" >/dev/null
}

# Wait for dbus
echo "Waiting for dbus"
until dbus_healthy >/dev/null; do
	sleep 1
done
echo "D-Bus socket ready"

# Start service
exec "$*" &
pid=$!
sleep 1

trap 'kill $pid' TERM INT

# Add unit arguments
if [ -n "${MOCK_SYSTEMD_UNITS}" ]; then
	for u in ${MOCK_SYSTEMD_UNITS}; do
		systemd_method MockAddUnit string:"$u"
		echo "Created unit: $u"
	done
fi

# Wait for the service to end
wait $pid
