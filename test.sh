#!/bin/sh
set -e
# ARGS=--pull
DISTRO=${1:-archlinux}
IMG=ego-$DISTRO

export DOCKER_BUILDKIT=1

docker build . ${2-} -f varia/Dockerfile.integration --build-arg=distro=$DISTRO -t $IMG
docker run --rm $IMG sh -c 'id && ego --sudo id'
