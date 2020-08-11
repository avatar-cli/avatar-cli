
# TODO: Remove default value
ARG DOCKER_IMAGE_VERSION

FROM docker:$DOCKER_IMAGE_VERSION-git

COPY ./target/release/avatar /usr/bin/avatar
