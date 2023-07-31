# mock-systemd-bus

Systemd D-Bus API mocks for testing. When ran, this service will run systemd in a container and replace dangerous services with mock versions.
At the moment this service will replace [systemd-login Manager](https://www.freedesktop.org/software/systemd/man/org.freedesktop.login1.html) object under `org.freedesktop.login1` to test interfacing with systemd units and system control.

This project has no goals of covering all risky systemd interfaces and features will be added as-needed.

## Usage

### docker-compose file

To use this service, setup a docker compose as below

```
version: '2.3'

services: 
  # The service setup
  mock-systemd:
    image: ghcr.io/balena-os/mock-systemd-bus
    volumes:
      - dbus:/shared/dbus
    environment:
      # Set this variable with the location of your shared bus socket
      DBUS_SYSTEM_BUS_ADDRESS: unix:path=/shared/dbus/system_bus_socket
      # Optionally set-up any mock units you need. Service files for these
      # Will be created on service start
      MOCK_SYSTEMD_UNITS: openvpn.service dummy.service

volumes:
  dbus:
    driver_opts:
      # Use tmpfs to avoid files remaining between runs
      type: tmpfs
      device: tmpfs 

```

### Creating and modifying system state

The service exposes an API on the same bus to manipulate system state.

**Reset the system mock power state**

```
dbus-send --system --print-reply \
		--dest=org.freedesktop.login1 \
		/org/freedesktop/login1 \
		"org.freedesktop.login1.Manager.MockReset
```

**Get the system mock power state**

```
dbus-send --system --print-reply \
		--dest=org.freedesktop.login1 \
		/org/freedesktop/login1 \
		org.freedesktop.DBus.Properties.Get string:org.freedesktop.login1.Manager "string:MockState"
```


## Environment variables

| Name               | Description                                                                                              | Default Value |
| ------------------ | ---------------------------------------------------------------------------------------------------------| ------------- |
| MOCK_SYSTEMD_UNITS | Space separated list of systemd unit names to create. The service will create these on the bus on start  | `''` (empty)  |
