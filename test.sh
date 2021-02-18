#!/bin/sh
# TODO: Rename script to something more descriptive

set -e
# ARGS=--pull
DISTRO=${1:-archlinux}
IMG=ego-$DISTRO

export DOCKER_BUILDKIT=1

if [ "$DISTRO" = ubuntu ]; then
  SYSTEMD=/bin/systemd
else
  SYSTEMD=/usr/lib/systemd/systemd
fi

docker build . ${2-} -f varia/Dockerfile.integration --build-arg=distro=$DISTRO -t $IMG
docker run --rm $IMG sh -c 'id && ego --sudo id'
docker run --rm \
  -e container=docker \
  --tmpfs /run \
  --tmpfs /tmp \
  -v /sys/fs/cgroup:/sys/fs/cgroup:ro \
  --cap-add SYS_ADMIN \
  $IMG "$SYSTEMD" quiet systemd.firstboot=off \
  systemd.setenv="CMD='id && mkdir -p /run/user/0 && XDG_RUNTIME_DIR=/run/user/0 ego --machinectl id'"
