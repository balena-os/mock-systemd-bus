#!/bin/bash

set -e

PS4='$LINENO: '

red() {
	printf "\033[31m%s\033[0m\n" "$1"
}

green() {
	printf "\033[32m%s\033[0m\n" "$1"
}

expect() {
	{ set +x; } 2>/dev/null
	cmd=""
	for arg in "$@"; do
		cmd="${cmd}'${arg}' "
	done

	if ! "$@"; then
		red "Test failed: $cmd"
		exit 1
	fi
	set -x
}

is_equal() {
	test "$1" = "$2"
}

parse() {
	input=$(cat)
	echo "$input" | tail -n +2 | sed 's/^ *//g' | tr -d '\n\r'
}

systemd_method() {
	{ set +x; } 2>/dev/null
	method=$1
	shift
	dbus-send --system --print-reply \
		--dest=org.freedesktop.systemd1 \
		/org/freedesktop/systemd1 \
		"org.freedesktop.systemd1.Manager.$method" "$@" | parse
	set -x
}

systemd_unit_prop() {
	{ set +x; } 2>/dev/null
	dbus-send --system --print-reply \
		--dest=org.freedesktop.systemd1 \
		"$1" \
		org.freedesktop.DBus.Properties.Get string:org.freedesktop.systemd1.Unit "string:$2" | parse
	set -x
}

login_method() {
	{ set +x; } 2>/dev/null
	method=$1
	shift
	dbus-send --system --print-reply \
		--dest=org.freedesktop.login1 \
		/org/freedesktop/login1 \
		"org.freedesktop.login1.Manager.$method" "$@" | parse
	set -x
}

login_prop() {
	{ set +x; } 2>/dev/null
	dbus-send --system --print-reply \
		--dest=org.freedesktop.login1 \
		/org/freedesktop/login1 \
		org.freedesktop.DBus.Properties.Get string:org.freedesktop.login1.Manager "string:$1" | parse
	set -x
}

cleanup() {
	login_method MockReset >/dev/null
}

set_abort_timer() {
	sleep "$1"
	# Send a USR2 signal to the given pid after the timeout happens
	kill -USR2 "$2"
}

timeout=30
abort_if_not_ready() {
	# If the timeout is reached and the required services are not ready, it probably
	# means something went wrong so we terminate the program with an error
	echo "Something happened, failed to start in ${timeout}s" >&2
	exit 1
}

trap 'abort_if_not_ready' USR2
set_abort_timer "$timeout" $$ &
timer_pid=$!

dbus_healthy() {
	dbus-send --system --print-reply \
		--dest=org.freedesktop.login1 \
		/org/freedesktop/login1 \
		org.freedesktop.DBus.Properties.Get string:org.freedesktop.login1.Manager string:MockState
}

# Wait for dbus
echo "Waiting for dbus"
until dbus_healthy >/dev/null; do
	sleep 1
done
kill $timer_pid
echo "D-Bus socket ready"

# Cleanup if any of the tests fail
trap 'cleanup' EXIT

# Start tracing here
set -x

expect is_equal "$(systemd_method GetUnit string:dummy.service)" 'object path "/org/freedesktop/systemd1/unit/dummy_2eservice"'
expect is_equal "$(systemd_unit_prop /org/freedesktop/systemd1/unit/dummy_2eservice ActiveState)" 'variant       string "active"'
expect systemd_method StopUnit string:dummy.service string:fail
expect is_equal "$(systemd_unit_prop /org/freedesktop/systemd1/unit/dummy_2eservice ActiveState)" 'variant       string "inactive"'
expect systemd_method StartUnit string:dummy.service string:fail
expect is_equal "$(systemd_unit_prop /org/freedesktop/systemd1/unit/dummy_2eservice ActiveState)" 'variant       string "active"'
expect systemd_method RestartUnit string:dummy.service string:fail
expect is_equal "$(systemd_unit_prop /org/freedesktop/systemd1/unit/dummy_2eservice ActiveState)" 'variant       string "active"'
expect is_equal "$(systemd_unit_prop /org/freedesktop/systemd1/unit/dummy_2eservice PartOf)" 'variant       array []'
expect is_equal "$(login_prop MockState)" 'variant       string "ready"'
expect is_equal "$(login_method PowerOff boolean:false)" ''
expect is_equal "$(login_prop MockState)" 'variant       string "off"'
expect is_equal "$(login_method Reboot boolean:false)" ''
expect is_equal "$(login_prop MockState)" 'variant       string "rebooting"'

{ set +x; } 2>/dev/null

green "All tests passed!"
