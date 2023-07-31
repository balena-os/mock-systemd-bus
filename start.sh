#!/bin/bash
set -m

# We hardcode the path here as we need to ensure that it works
# with our dbus.socket config
DBUS_SYSTEM_BUS_ADDRESS=${DBUS_SYSTEM_BUS_ADDRESS:-unix:path=/shared/dbus/system_bus_socket}
DBUS_SYSTEM_PATH=${DBUS_SYSTEM_BUS_ADDRESS/unix:path=/}

# We override the dbus socket
mkdir -p /etc/systemd/system/dbus.socket.d
cat <<-EOF >/etc/systemd/system/dbus.socket.d/override.conf
	[Socket]
	ListenStream=${DBUS_SYSTEM_PATH}
EOF

# User defined list of fake systemd units
MOCK_SYSTEMD_UNITS=${MOCK_SYSTEMD_UNITS:-""}

# Create files for each unit
# we write this in the container layer so it
# gets erased on container re-create
if [ -n "${MOCK_SYSTEMD_UNITS}" ]; then
	for u in ${MOCK_SYSTEMD_UNITS}; do
		cat <<-EOF >"/etc/systemd/system/${u}"
			[Unit]
			Description=${u}

			[Service]
			EnvironmentFile=/etc/docker.env
			ExecStart=sleep infinity
			StandardOutput=inherit
			StandardError=inherit
			TTYPath=/dev/console
			Restart=on-failure

			[Install]
			WantedBy=basic.target
		EOF
		systemctl enable "${u}"
	done
	unset MOCK_SYSTEMD_UNITS
fi

RESET='\033[0;0m'
GREEN='\033[0;32m'
echo -e "${GREEN}Systemd init system enabled.${RESET}"

# systemd causes a POLLHUP for console FD to occur
# on startup once all other process have stopped.
# We need this sleep to ensure this doesn't occur, else
# logging to the console will not work.
sleep infinity &
for var in $(compgen -e); do
	printf '%q=%q\n' "$var" "${!var}"
done >/etc/docker.env
exec /sbin/init
