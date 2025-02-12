# Use debian:testing since we need rust v1.64
FROM debian:testing-slim AS testing

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
     dbus curl ca-certificates build-essential git

WORKDIR /rust

# Install rust. Recommended install does not work in all docker
# architectures, as it fails to detect the right architecture when using
# buildx
COPY install-rust.sh ./
RUN ./install-rust.sh

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN . "$HOME/.cargo/env" && \
	# See https://blog.rust-lang.org/inside-rust/2023/01/30/cargo-sparse-protocol.html
	CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse cargo build --release --verbose

COPY run-tests.sh /
CMD ["/run-tests.sh"]

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    dbus systemd-sysv \
    && rm -rf /var/lib/apt/lists/*

# We want to use the multi-user.target not graphical.target
RUN systemctl set-default multi-user.target \
	# We never want these to run in a container
	&& systemctl mask \
		apt-daily.timer \
		apt-daily-upgrade.timer \
		dev-hugepages.mount \
		dev-mqueue.mount \
		sys-fs-fuse-connections.mount \
		sys-kernel-config.mount \
		sys-kernel-debug.mount \
		display-manager.service \
		getty@.service \
		systemd-logind.service \
		systemd-remount-fs.service \
		getty.target \
		graphical.target

# Copy the start script
COPY start.sh /

# Install and enable the mock-logind service
COPY --from=testing /app/target/release/mock-logind /usr/bin/
COPY mock-logind.service /etc/systemd/system
RUN systemctl enable mock-logind.service

CMD ["/start.sh"]

# Set the stop signal to SIGRTMIN+3 which systemd understands as the signal to halt
STOPSIGNAL SIGRTMIN+3
