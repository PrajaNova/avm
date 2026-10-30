# End-to-end tests on a clean Ubuntu. Two targets:
#   local (default): avm-bin built from this checkout, installed by this
#                    checkout's install.sh, so a PR tests its own code.
#   release:         nothing prebuilt; the tests install the published avm.
# amd64 because Google ships Android's Linux tools for x86_64 only (Docker
# on Apple Silicon runs this through Rosetta).

FROM --platform=linux/amd64 rust:1 AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY plugins plugins
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --workspace \
 && mkdir /dist \
 && for tool in node java python android; do mkdir -p /dist/plugins/avm-plugin-$tool/bin; cp target/release/avm-plugin-$tool /dist/plugins/avm-plugin-$tool/bin/avm-plugin; done \
 && tar -C target/release -czf /dist/avm_linux_amd64.tar.gz avm-bin \
 && cd /dist && sha256sum avm_* > checksums.txt

FROM --platform=linux/amd64 ubuntu:24.04 AS release
RUN apt-get update \
 && apt-get install -y --no-install-recommends curl ca-certificates tar gzip unzip git \
 && rm -rf /var/lib/apt/lists/*
# A normal user, like a real install (no root, no sudo).
RUN useradd -m tester
COPY --chown=tester crates/avm-cli/tests/shell/suites/ /suites/
USER tester
WORKDIR /home/tester
ENV AVM_E2E_INSTALL=release
# No update notices or GitHub checks in the suites (core.sh tests them).
ENV AVM_NO_UPDATE_CHECK=1
ENTRYPOINT ["bash", "/suites/run-all.sh"]

FROM release AS local
COPY --from=build --chown=tester /dist /opt/dist
COPY install.sh /opt/install.sh
ENV AVM_E2E_INSTALL=local

# playground: an interactive shell with this checkout's avm installed the way a
# user does it (install.sh) and every plugin built from the checkout. Used by
# docker-compose.yml; throwaway like the test image.
FROM local AS playground
RUN AVM_DOWNLOAD_BASE=file:///opt/dist bash /opt/install.sh \
 && for tool in node java python android; do \
      mkdir -p ~/.avm/plugins/avm-plugin-$tool/bin \
      && cp /opt/dist/plugins/avm-plugin-$tool/bin/avm-plugin ~/.avm/plugins/avm-plugin-$tool/bin/; \
    done \
 && ~/.local/bin/avm-bin shims install
ENTRYPOINT ["bash", "-l"]
