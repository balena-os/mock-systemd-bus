FROM alpine as testing

RUN apk add --update --no-cache build-base rust cargo dbus

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

COPY run-tests.sh /
CMD ["/run-tests.sh"]

FROM alpine

RUN apk add --update --no-cache dbus libstdc++

COPY --from=testing /app/target/release/mock-systemd-bus /mock-systemd-bus
COPY start.sh /

CMD ["/start.sh", "/mock-systemd-bus"]
