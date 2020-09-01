
FROM registry.gitlab.com/avatar-cli/avatar-cli/docker:git

COPY ./target/release/avatar /usr/bin/avatar
