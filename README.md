# mock-systemd-bus

Systemd D-Bus API mocks for testing. When ran, this service will expose a mock [systemd Manager](https://www.freedesktop.org/software/systemd/man/org.freedesktop.systemd1.html) object under `org.freedesktop.systemd1` and a mock [systemd-login Manager](https://www.freedesktop.org/software/systemd/man/org.freedesktop.login1.html) object under `org.freedesktop.login1` to test interfacing with systemd units and system control.

This project has no goals of being an exhaustive simulation of systemd behavior and features will be added as-needed.

## Usage

### docker-compose file

To use this service, you'll need to bring your own bus (BYOB), the following docker-compose shows how to use this with the [dbus block](https://github.com/balena-labs-projects/dbus).

```
version: '2.3'

services: 
  # The service setup
  mock-systemd:
    image: ghcr.io/balena-os/mock-systemd-bus
    depends_on:
       - dbus
    volumes:
      - dbus:/run/dbus
    environment:
      # Set this variable with the location of your bus socket
      # DO NOT try to use your actual system bus
      DBUS_SYSTEM_BUS_ADDRESS: unix:path=/run/dbus/system_bus_socket
      # Optionally set-up any fake units you need
      FAKE_SYSTEMD_UNITS: openvpn.service dummy.service

  # BYOB
  dbus:
    image: balenablocks/dbus
    environment:
      DBUS_CONFIG: session.conf
      DBUS_ADDRESS: unix:path=/run/dbus/system_bus_socket
    volumes:
      - dbus:/run/dbus

volumes:
  dbus:
    driver_opts:
      # Use tmpfs to avoid files remaining between runs
      type: tmpfs
      device: tmpfs 

```

### Creating and modifying system state

The service exposes an API on the same bus to manipulate system state.

**Create a mock unit**

```
dbus-send --system --print-reply \
		--dest=org.freedesktop.systemd1 \
		/org/freedesktop/systemd1 \
		"org.freedesktop.systemd1.Manager.MockAddUnit "string:<my-unit>"
```


**Remove a mock unit**
```
dbus-send --system --print-reply \
		--dest=org.freedesktop.systemd1 \
		/org/freedesktop/systemd1 \
		"org.freedesktop.systemd1.Manager.MockDelUnit "string:<my-unit>"
```

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
