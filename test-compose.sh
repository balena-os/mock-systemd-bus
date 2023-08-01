#!/bin/sh

cleanup() {
	docker compose -f docker-compose.yml -f docker-compose.test.yml down --volumes
}

trap cleanup EXIT

export BUILDKIT_PROGRESS=plain
docker compose -f docker-compose.yml -f docker-compose.test.yml up --build --remove-orphans --exit-code-from=sut
