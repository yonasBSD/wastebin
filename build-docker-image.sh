#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail

VERSION=$(git describe --tags)

docker build --target amd64 -t quxfoo/wastebin:${VERSION}-amd64 .
docker build --target arm64 -t quxfoo/wastebin:${VERSION}-arm64 .

docker push quxfoo/wastebin:${VERSION}-amd64
docker push quxfoo/wastebin:${VERSION}-arm64

docker manifest create quxfoo/wastebin:${VERSION} \
    quxfoo/wastebin:${VERSION}-amd64 \
    quxfoo/wastebin:${VERSION}-arm64

docker manifest annotate quxfoo/wastebin:${VERSION} \
    quxfoo/wastebin:${VERSION}-arm64 \
    --os linux \
    --arch arm64

docker manifest rm quxfoo/wastebin:latest 2>/dev/null || true

docker manifest create quxfoo/wastebin:latest \
    quxfoo/wastebin:${VERSION}-amd64 \
    quxfoo/wastebin:${VERSION}-arm64

docker manifest annotate quxfoo/wastebin:latest \
    quxfoo/wastebin:${VERSION}-arm64 \
    --os linux \
    --arch arm64

docker manifest inspect quxfoo/wastebin:${VERSION}

docker manifest push quxfoo/wastebin:${VERSION}
docker manifest push quxfoo/wastebin:latest
